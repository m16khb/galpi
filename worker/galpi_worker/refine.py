"""Transcript refinement through the Z.ai coding-plan chat endpoint."""

import json
import os
import urllib.error
import urllib.request
from pathlib import Path
from typing import cast

from .protocol import EventWriter

API_KEY_VARIABLE = "GALPI_ASSISTANT_API_KEY"
BASE_URL_VARIABLE = "GALPI_ASSISTANT_BASE_URL"
DEFAULT_BASE_URL = "https://api.z.ai/api/coding/paas/v4"
DEFAULT_MODEL = "glm-5.3"
MAX_OUTPUT_TOKENS = 16384
REQUEST_TIMEOUT_SECONDS = 600

SYSTEM_PROMPT = """당신은 한국어 회의 기록을 실행 가능한 회의록으로 가공하는 전문가입니다.

규칙:
- 시간순 받아쓰기 요약을 만들지 않습니다. 회의가 끝난 뒤 남아야 하는 결정, 담당, 기한, 리스크, 후속 확인을 남깁니다.
- 결정사항에는 확실한 내용만 씁니다. 불확실한 내용은 `리스크/열린 질문`이나 보정 부록에 씁니다.
- 사전 정보에 있는 제품명, 서비스명, 팀 구성원, 별칭, 도메인 용어를 우선 적용해 오인식된 표현을 보정합니다.
- 근거 없이 `SPEAKER_00` 같은 화자 라벨을 실명으로 바꾸지 않습니다. 근거가 부족하면 `{이름} 추정`으로 쓰고 보정 부록에 신뢰도를 남깁니다.
- 실행 항목은 산출물 중심으로 씁니다. 기한이 전사본에 없으면 `미정`으로 씁니다.
- 원문 전사본은 별도 파일로 이미 보관되므로 회의록 본문에 다시 붙여넣지 않습니다.
- 출력은 Markdown 문서 하나이며, 코드펜스로 전체를 감싸지 않습니다.

문서 구조는 다음 순서를 그대로 따릅니다.

# {날짜 또는 미정} [{주제}] {제목}

## TL;DR
- 2~4개 항목

## 결정사항
- **{결정 제목}**
  - 내용:
  - 근거:
  - 영향:
  - 결정자/동의자:

## 액션 보드
- [ ] {담당}: {산출물}. 기한: {날짜 또는 미정}.

## 주제별 논의
### {주제}
- 배경:
- 논점:
- 정리:

## 후속 확인
- {확인할 것}. 담당: {이름 또는 미정}. 확인 위치: {위치 또는 미정}.

## 리스크/열린 질문
- **{제목}**
  - 내용:
  - 확인 담당:
  - 확인 방법:

---

## 보정 부록
### 용어 보정
- `{원문 표현}` -> `{보정 표현}`. 근거: {근거}. 신뢰도: {높음/중간/낮음}.

### 화자 보정
- `{화자 라벨}` -> `{이름 또는 확인 필요}`. 근거: {근거}. 신뢰도: {높음/중간/낮음}.
"""

NO_BACKGROUND = "(등록된 사전 정보가 없습니다. 전사본에 있는 근거만 사용하세요.)"


def build_messages(transcript: str, background: str) -> list[dict[str, str]]:
    """Build the chat messages for one refinement request."""

    if not transcript.strip():
        raise ValueError("transcript is empty")
    context = background.strip() or NO_BACKGROUND
    user = (
        "다음은 회의 참석자, 제품/서비스, 용어에 대한 사전 정보입니다.\n"
        "<사전정보>\n"
        f"{context}\n"
        "</사전정보>\n\n"
        "다음은 화자분리된 회의 전사본입니다.\n"
        "<전사본>\n"
        f"{transcript.strip()}\n"
        "</전사본>\n\n"
        "위 규칙에 따라 회의록 Markdown 문서를 작성하세요."
    )
    return [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": user},
    ]


def extract_minutes(payload: object) -> str:
    """Read the assistant message out of a chat-completion response."""

    if not isinstance(payload, dict):
        raise TypeError("assistant response was not a JSON object")
    body = cast(dict[str, object], payload)
    choices = body.get("choices")
    if not isinstance(choices, list) or not choices:
        error = body.get("error")
        detail = (
            json.dumps(error, ensure_ascii=False) if error is not None else "no choices"
        )
        raise RuntimeError(f"assistant response had no choices: {detail}")
    first = cast(list[object], choices)[0]
    if not isinstance(first, dict):
        raise TypeError("assistant choice was not a JSON object")
    message = cast(dict[str, object], first).get("message")
    if not isinstance(message, dict):
        raise TypeError("assistant choice had no message")
    content = cast(dict[str, object], message).get("content")
    if not isinstance(content, str) or not content.strip():
        raise RuntimeError("assistant returned an empty message")
    return strip_document_fence(content.strip())


def strip_document_fence(document: str) -> str:
    """Remove a code fence that wraps the whole document."""

    if not document.startswith("```"):
        return document
    lines = document.splitlines()
    if len(lines) < 2 or lines[-1].strip() != "```":
        return document
    return "\n".join(lines[1:-1]).strip()


def request_minutes(messages: list[dict[str, str]], model: str, api_key: str) -> str:
    """Call the chat-completions endpoint and return the refined document."""

    base_url = os.environ.get(BASE_URL_VARIABLE, DEFAULT_BASE_URL).rstrip("/")
    body = json.dumps(
        {
            "model": model,
            "messages": messages,
            "stream": False,
            "temperature": 0.2,
            "max_tokens": MAX_OUTPUT_TOKENS,
        },
        ensure_ascii=False,
    ).encode("utf-8")
    request = urllib.request.Request(
        f"{base_url}/chat/completions",
        data=body,
        method="POST",
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "Accept": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(
            request, timeout=REQUEST_TIMEOUT_SECONDS
        ) as response:
            payload = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")[:500]
        raise RuntimeError(
            f"assistant request failed ({error.code}): {detail}"
        ) from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"assistant request failed: {error.reason}") from error
    return extract_minutes(payload)


def write_text_atomic(path: Path, document: str) -> None:
    temporary = path.with_suffix(f"{path.suffix}.tmp")
    temporary.write_text(f"{document}\n", encoding="utf-8")
    temporary.replace(path)


def refine(
    transcript_path: Path,
    output_path: Path,
    background_path: Path | None,
    model: str,
    events: EventWriter,
) -> None:
    """Refine a speaker-labeled transcript into Korean meeting minutes."""

    api_key = os.environ.get(API_KEY_VARIABLE, "").strip()
    if not api_key:
        raise ValueError("assistant API key is not configured")
    events.emit(
        "phase", phase="refining", percent=10.0, message="전사본을 읽는 중입니다."
    )
    transcript = transcript_path.read_text(encoding="utf-8")
    background = (
        background_path.read_text(encoding="utf-8")
        if background_path is not None and background_path.is_file()
        else ""
    )
    events.emit(
        "phase",
        phase="refining",
        percent=35.0,
        message=f"{model} 모델로 회의록을 작성하는 중입니다.",
    )
    document = request_minutes(build_messages(transcript, background), model, api_key)
    events.emit(
        "phase", phase="writing", percent=90.0, message="회의록을 저장하는 중입니다."
    )
    write_text_atomic(output_path, document)
    events.emit("refined", minutes=str(output_path))
