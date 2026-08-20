"""Transcript refinement through the Z.ai coding-plan chat endpoint."""

import json
import os
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from .protocol import EventWriter

API_KEY_VARIABLE = "GALPI_ASSISTANT_API_KEY"
BASE_URL_VARIABLE = "GALPI_ASSISTANT_BASE_URL"
DEFAULT_BASE_URL = "https://api.z.ai/api/coding/paas/v4"
DEFAULT_MODEL = "glm-5.3"
MAX_OUTPUT_TOKENS = 16384
REQUEST_TIMEOUT_SECONDS = 600

SYSTEM_PROMPT = """당신은 한국어 회의 기록을 팀의 실행 기억으로 증강하는 전문가입니다. 회의가 끝난 뒤에도 남아 있어야 하는 것 — 결정, 담당, 기한, 리스크, 후속 확인 — 만 남기고, 나머지는 훈증시킵니다.

원칙:
- 시간순 받아쓰기 요약을 만들지 않습니다. 읽는 사람이 전사본을 열지 않고도 회의를 이해할 수 있게 실행 정보 우선으로 구성합니다.
- 스캔 우선 설계: TL;DR → 결정사항 → 액션 보드 → 주제별 논의 → 후속 확인 → 리스크/열린 질문 → 보정 부록 순서를 그대로 지킵니다. 읽는 사람은 위에서부터 훑기만 해도 흐름이 보여야 합니다.
- 결정사항에는 확실하고 귀속 가능한 내용만 씁니다. 불확실한 내용은 `리스크/열린 질문`과 보정 부록으로 격리합니다. 결정 하나를 한 문장에 우겨넣지 않고 내용/근거/영향/결정자/상태 필드로 나눠 씁니다.
- 액션은 산출물 중심으로 씁니다. `검토한다` 대신 `GitLab 이슈 본문에 검증 체크리스트를 추가한다`처럼 구체적으로. 기한이 전사본에 없으면 `미정`이라고 씁니다. 담당은 가능한 한 한 사람으로 특정하고, 팀 소유로만 확인되면 `미정`으로 두고 열린 질문에 추가합니다.
- 근거 없이 `SPEAKER_00` 같은 화자 라벨을 실명으로 바꾸지 않습니다. 근거가 부족하면 `{이름} 추정`으로 쓰고 보정 부록에 신뢰도를 남깁니다. 담당/결정 필드에는 확인된 최선의 역할을 쓰되, 화자 매핑 불확실성은 부록에 그대로 둡니다.
- 참석자 명단이 주어지면 화자 라벨의 실명 후보는 그 명단 안에서만 찾습니다. 명단에 없는 사람을 추정해 넣지 않고, 명단의 별칭은 대표 이름으로 통일합니다. 참석자 설명에 담긴 담당 업무는 액션 보드의 담당 매칭에 활용합니다.
- 사전 정보의 제품명, 서비스명, 팀 구성원, 별칭, 도메인 용어를 우선 적용해 오인식 표현을 보정합니다. 단어집 등록 용어는 표기를 그대로 따르고, 단어집 기준으로 보정한 표현은 모두 용어 보정에 기록합니다.
- 리스크/열린 질문도 제목 있는 불릿으로 내용/확인 담당/확인 방법/상태 필드를 나눠 씁니다. 한 줄에 우겨넣지 않습니다.
- 민감정보(토큰·비밀번호·API 키)가 전사본에 있으면 `[민감정보 생략]`으로 가립니다.
- 원문 전사본은 별도 파일로 이미 보관되므로 회의록 본문에 붙여넣지 않습니다.
- 출력은 Markdown 문서 하나이며, 코드펜스로 전체를 감싸지 않습니다.

문서 구조는 다음 순서를 그대로 따릅니다. 각 섹션 제목은 본문을 읽지 않아도 흐름이 보이게 만듭니다.

# {날짜 또는 미정} [{주제}] {제목}

> 회의일 {날짜 또는 미정} · 참석 {요약} · Source 화자분리 전사본 · Status {Draft/완료/확인 필요}

## TL;DR
- {회의 결론 또는 방향성 1줄}
- {우선순위 또는 실행 흐름}
- {주요 follow-up 또는 리스크}
(여러 독립 결론이 있으면 2~4개 불릿으로 씁니다)

## 결정사항
- **{짧은 결정 제목}**
  - 내용: {확정된 결정}
  - 근거: {전사본 근거, 가능하면 시각/화자}
  - 영향: {영향 범위}
  - 결정자/동의자: {이름 또는 미정}
  - 상태: 확정

## 액션 보드
- [ ] {담당}: {산출물 중심 작업}. 기한: {날짜 또는 미정}. Tracking: {추적 위치 또는 미정}.

## 주제별 논의
### {주제}
- 배경: {왜 논의했는가}
- 논점: {핵심 내용}
- 정리: {현재 상태}

## 후속 확인
- {확인할 것}. 담당: {이름 또는 미정}. 확인 위치: {위치 또는 미정}. 기한: {날짜 또는 미정}.

## 리스크/열린 질문
- **{리스크 또는 질문 제목}**
  - 내용: {불명확한 점}
  - 확인 담당: {이름 또는 미정}
  - 확인 방법: {확인 경로}
  - 상태: 확인 필요

---

## 보정 부록
### 용어 보정
- `{원문 표현}` -> `{보정 표현}`. 근거: {문맥/단어집/사전 정보}. 신뢰도: {높음/중간/낮음}.

### 화자 보정
- `{화자 라벨}` -> `{이름 또는 확인 필요}`. 근거: {근거}. 신뢰도: {높음/중간/낮음}.
"""

NO_BACKGROUND = "(등록된 사전 정보가 없습니다. 전사본에 있는 근거만 사용하세요.)"
NO_PARTICIPANTS = "(선택된 참석자가 없습니다. 전사본에 있는 근거만 사용하세요.)"
NO_GLOSSARY = "(등록된 용어가 없습니다. 전사본에 있는 근거만 사용하세요.)"


@dataclass(frozen=True)
class Participant:
    """One meeting attendee selected from the saved roster."""

    name: str
    team: str | None
    role: str | None
    description: str | None
    aliases: tuple[str, ...]


@dataclass(frozen=True)
class GlossaryEntry:
    """One saved glossary term applied to every refinement."""

    term: str
    description: str | None


def optional_text(value: object) -> str | None:
    """Trim a roster field to text, treating blank as absent."""

    if isinstance(value, str) and value.strip():
        return value.strip()
    return None


def parse_participants(payload: object) -> list[Participant]:
    """Convert the roster JSON handed over by the host into typed participants."""

    if not isinstance(payload, list):
        raise TypeError("participants payload was not a JSON array")
    participants: list[Participant] = []
    for entry in cast(list[object], payload):
        if not isinstance(entry, dict):
            raise TypeError("participant entry was not a JSON object")
        fields = cast(dict[str, object], entry)
        name = fields.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError("participant entry had no name")
        team = optional_text(fields.get("team"))
        role = optional_text(fields.get("role"))
        description = optional_text(fields.get("description"))
        raw_aliases = fields.get("aliases")
        aliases = (
            tuple(
                alias.strip()
                for alias in cast(list[object], raw_aliases)
                if isinstance(alias, str) and alias.strip()
            )
            if isinstance(raw_aliases, list)
            else ()
        )
        participants.append(
            Participant(
                name=name.strip(),
                team=team,
                role=role,
                description=description,
                aliases=aliases,
            )
        )
    return participants


def parse_glossary(payload: object) -> list[GlossaryEntry]:
    """Convert the glossary JSON handed over by the host into typed entries."""

    if not isinstance(payload, list):
        raise TypeError("glossary payload was not a JSON array")
    entries: list[GlossaryEntry] = []
    for entry in cast(list[object], payload):
        if not isinstance(entry, dict):
            raise TypeError("glossary entry was not a JSON object")
        fields = cast(dict[str, object], entry)
        term = fields.get("term")
        if not isinstance(term, str) or not term.strip():
            raise ValueError("glossary entry had no term")
        entries.append(
            GlossaryEntry(
                term=term.strip(), description=optional_text(fields.get("description"))
            )
        )
    return entries


def render_participants(participants: list[Participant]) -> str:
    """Render the attendee roster as one prompt block."""

    if not participants:
        return NO_PARTICIPANTS
    lines: list[str] = []
    for participant in participants:
        detail = " · ".join(
            part for part in (participant.team, participant.role) if part is not None
        )
        line = f"- {participant.name}" + (f" ({detail})" if detail else "")
        if participant.aliases:
            line += f" / 별칭: {', '.join(participant.aliases)}"
        if participant.description is not None:
            line += f" / 설명: {participant.description}"
        lines.append(line)
    return "\n".join(lines)


def render_glossary(entries: list[GlossaryEntry]) -> str:
    """Render the glossary as one prompt block."""

    if not entries:
        return NO_GLOSSARY
    return "\n".join(
        f"- {entry.term}: {entry.description}"
        if entry.description is not None
        else f"- {entry.term}"
        for entry in entries
    )


def build_messages(
    transcript: str,
    background: str,
    participants: list[Participant],
    glossary: list[GlossaryEntry],
) -> list[dict[str, str]]:
    """Build the chat messages for one refinement request."""

    if not transcript.strip():
        raise ValueError("transcript is empty")
    context = background.strip() or NO_BACKGROUND
    user = (
        "다음은 회의 참석자, 제품/서비스, 용어에 대한 사전 정보입니다.\n"
        "<사전정보>\n"
        f"{context}\n"
        "</사전정보>\n\n"
        "다음은 이 회의에 참석한 사람들입니다. 화자 실명 후보는 이 명단 안에서만 찾으세요.\n"
        "<참석자>\n"
        f"{render_participants(participants)}\n"
        "</참석자>\n\n"
        "다음은 이 팀이 자주 쓰는 용어의 단어집입니다. 등록된 표기를 그대로 따르세요.\n"
        "<단어집>\n"
        f"{render_glossary(glossary)}\n"
        "</단어집>\n\n"
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
    participants_path: Path | None,
    glossary_path: Path | None,
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
    participants = (
        parse_participants(json.loads(participants_path.read_text(encoding="utf-8")))
        if participants_path is not None and participants_path.is_file()
        else []
    )
    glossary = (
        parse_glossary(json.loads(glossary_path.read_text(encoding="utf-8")))
        if glossary_path is not None and glossary_path.is_file()
        else []
    )
    events.emit(
        "phase",
        phase="refining",
        percent=35.0,
        message=f"{model} 모델로 회의록을 작성하는 중입니다.",
    )
    document = request_minutes(
        build_messages(transcript, background, participants, glossary), model, api_key
    )
    events.emit(
        "phase", phase="writing", percent=90.0, message="회의록을 저장하는 중입니다."
    )
    write_text_atomic(output_path, document)
    events.emit("refined", minutes=str(output_path))
