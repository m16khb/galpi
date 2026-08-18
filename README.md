# Galpi

Galpi(갈피)는 회의를 **Apple Silicon Mac 안에서 녹음하고 전사하는 Tauri 데스크톱 앱**입니다.

- 앱 안에서 CoreAudio 마이크 녹음
- 기존 `m4a`, `mp3`, `wav`, `mp4`, `mov`, `aac`, `flac`, `ogg` 파일 전사
- WhisperX `large-v3-turbo` 한국어 전사
- 한국어 문장 정렬과 pyannote 화자분리
- 정확한 화자 수 또는 화자 수 범위 힌트
- 장기 무음 뒤의 저신뢰 환각 필터링
- SRT, 화자별 텍스트, 정렬 체크포인트 생성

`meeting-transcribe`나 별도의 Python 환경을 미리 설치할 필요가 없습니다. Galpi가 최초 실행 시 앱 전용 Python 3.12, WhisperX, ffmpeg와 모델을 직접 준비합니다.

## 사용 방법

### 최초 준비

1. Galpi를 실행합니다.
2. 화자분리 모델 접근 승인이 필요한 경우 우측 상단 설정에서 Hugging Face 토큰을 저장합니다.
3. **로컬 엔진 준비**를 누릅니다.
4. 엔진, 모델, ffmpeg가 모두 `준비됨`으로 표시될 때까지 기다립니다.

최초 준비는 약 3GB의 모델을 내려받을 수 있습니다. 저장한 토큰은 모델 준비 프로세스에 자동으로 전달되며, 준비가 끝난 뒤에도 같은 앱 데이터 폴더와 모델 캐시를 재사용합니다.

기존 WhisperX가 사용자 표준 Hugging Face 캐시(`~/.cache/huggingface/hub`)에 동일한 모델을 이미 내려받았다면, Galpi는 고정된 세 모델 저장소의 파일만 앱 전용 캐시에 안전하게 재사용합니다. 토큰이나 다른 저장소는 가져오지 않습니다.

#### Hugging Face 토큰 권한과 발급

화자분리 모델을 새 Mac에 처음 내려받을 때만 토큰이 필요합니다.

1. Hugging Face 계정으로 로그인합니다.
2. [`pyannote/speaker-diarization-community-1`](https://huggingface.co/pyannote/speaker-diarization-community-1)에서 이용 조건에 동의하고 접근 승인을 받습니다.
3. [Access Tokens](https://huggingface.co/settings/tokens)에서 **Create new token**을 누릅니다.
4. **Fine-grained** 토큰을 선택하고 `pyannote/speaker-diarization-community-1` 저장소 콘텐츠에 대한 **Read** 권한만 허용합니다.
5. 쓰기 권한이나 Inference Providers 권한은 추가하지 않습니다.
6. `hf_`로 시작하는 토큰을 Galpi 우측 상단의 톱니바퀴 설정에서 저장하고 로컬 엔진 준비를 시작합니다.

저장한 토큰은 눈 모양 버튼으로 표시하거나 숨길 수 있고, **저장된 토큰 지우기**로 삭제할 수 있습니다. 값은 이 Mac의 Galpi Application Support 아래 `settings.json`에 `0600` 권한으로 보관되며 macOS Keychain 암호화를 사용하지 않습니다. 각 Mac은 앱 전용 설정과 모델 캐시를 별도로 사용하므로, 새 Mac에서는 같은 절차가 다시 필요할 수 있습니다.

### 다른 Mac에서 실행

배포 DMG는 **macOS 14 이상 Apple Silicon(M1 이상)** 용입니다. 대상 Mac에는 Python, Homebrew, ffmpeg, WhisperX, `meeting-transcribe`, Rust 또는 Bun을 설치할 필요가 없습니다. 앱에 검증된 arm64 `uv` 실행 파일과 Python worker가 포함되며, 최초 실행 때 사용자별 Application Support 디렉터리에 Python 3.12와 모든 런타임을 설치합니다.

새 Mac에서는 인터넷 연결, 모델 다운로드 공간, Hugging Face 모델 접근 승인이 필요합니다. Intel Mac과 Windows/Linux는 현재 배포 대상이 아닙니다.

### Apple Silicon 가속

- WhisperX의 `faster-whisper`/CTranslate2 전사 단계는 현재 Metal(MPS) backend를 제공하지 않으므로 Apple Accelerate가 적용된 arm64 CPU `int8` 경로를 사용합니다.
- PyTorch 기반 한국어 문장 정렬과 pyannote 화자분리는 Apple GPU의 MPS를 사용합니다.
- MPS 연산을 지원하지 않는 모델·Mac에서는 해당 단계만 CPU로 자동 재시도하며, 앱 상세 로그에 fallback 이유를 남깁니다.

### 앱에서 바로 녹음

1. 결과를 저장할 출력 폴더를 확인하거나 변경합니다.
2. **마이크로 바로 녹음**을 누르고 macOS 마이크 권한을 허용합니다.
3. 회의가 끝나면 **정지**를 누릅니다.
4. 완성된 PCM WAV가 자동으로 전사 입력에 선택됩니다.
5. 화자 수 힌트를 고른 뒤 **전사 시작**을 누릅니다.

녹음은 CoreAudio 콜백에서 bounded queue로 넘겨 전용 WAV writer thread가 점진적으로 저장합니다. 전체 회의를 메모리에 보관하지 않습니다. **버리기**는 현재 녹음을 취소하고 부분 WAV를 삭제합니다.

현재 녹음 기능은 선택된 Mac 마이크 입력을 기록합니다. macOS 시스템 오디오를 직접 캡처하지는 않습니다.

### 기존 녹음 전사

1. **오디오 파일 선택**에서 회의 녹음을 고릅니다.
2. 출력 폴더를 확인합니다. 기본값은 `~/Downloads/whisperx-out`입니다.
3. 화자 수를 모르면 `자동`, 정확히 알면 `정확히`, 범위만 알면 `범위`를 선택합니다.
4. **전사 시작**을 누릅니다.

각 작업은 충돌을 막기 위해 독립된 출력 디렉터리를 사용합니다.

```text
<출력 폴더>/<파일명>-<작업 ID>/
├── <파일명>.srt
├── <파일명>_화자별.txt
└── <파일명>.aligned.v2.json
```

완료 화면에서 각 결과를 열거나 Finder에서 출력 폴더를 확인할 수 있습니다.

## 아키텍처

Rust 백엔드는 단일 crate 안에서 헥사고날 의존성 방향을 유지합니다.

```text
domain
  ↑
application (ports + use cases + job/recording lifecycle)
  ↑
adapters
  ├── inbound/tauri
  └── outbound
      ├── process (Python worker supervision)
      ├── recording (CoreAudio -> bounded queue -> WAV writer)
      ├── filesystem
      └── opener
  ↑
composition
```

Python worker는 WhisperX 추론과 산출물 생성을 담당하는 외부 어댑터입니다. Rust와는 버전이 지정된 JSONL 프로토콜로 통신합니다. 프론트엔드는 순수 상태 전이, Tauri IPC 어댑터, DOM 뷰를 분리합니다.

## 개발 환경

- macOS 14 이상
- Rust 1.85 이상
- Bun 1.3 이상
- `tauri-cli 2.11.4`

```bash
cargo install tauri-cli --version 2.11.4 --locked
bun install
bun run dev
```

## 프로덕션 빌드

```bash
bun run build
```

빌드 전에 저장소의 Python worker와 requirements가 Tauri resource 디렉터리로 동기화됩니다. 빌드 결과:

```text
src-tauri/target/release/bundle/macos/Galpi.app
src-tauri/target/release/bundle/dmg/Galpi_0.1.0_aarch64.dmg
```

`bun run build`는 먼저 Tauri `.app`을 만든 뒤 Finder AppleScript에 의존하지 않는 `hdiutil` 패키저로 DMG를 생성합니다. 배포 서명과 notarization은 Apple Developer 인증서 환경에서 별도로 수행해야 합니다.

## 검증

```bash
bun run check
bun test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
uvx ruff check worker
uvx ruff format --check worker
uvx basedpyright --pythonpath <WhisperX Python 경로>
PYTHONPATH=. python -m unittest worker.tests.test_core -v
```

## 로컬 데이터와 권한

- 음성과 전사 결과는 사용자가 선택한 로컬 폴더에만 기록됩니다.
- 모델은 Galpi 앱 데이터 디렉터리의 전용 Hugging Face 캐시에 저장됩니다.
- 마이크 권한 설명은 앱 번들의 `NSMicrophoneUsageDescription`에 포함됩니다.
- 녹음 권한은 `com.apple.security.device.audio-input` entitlement로 선언됩니다.
- 앱은 셸 문자열을 실행하지 않으며, 고정된 worker와 argv만 직접 실행합니다.
