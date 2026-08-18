export const appTemplate = `
  <div class="app-shell">
    <aside class="setup-rail" aria-label="진행 단계">
      <div class="brand-lockup">
        <span class="brand-mark" aria-hidden="true"><i class="ph ph-waveform"></i></span>
        <div><strong>갈피</strong><span>LOCAL TRANSCRIPTION</span></div>
      </div>
      <ol class="step-list">
        <li id="step-engine" data-state="current"><span>01</span><div><strong>엔진 준비</strong><small>Python · WhisperX</small></div></li>
        <li id="step-model" data-state="pending"><span>02</span><div><strong>모델 준비</strong><small>전사 · 정렬 · 화자분리</small></div></li>
        <li id="step-transcribe" data-state="pending"><span>03</span><div><strong>회의 전사</strong><small>오디오에서 결과까지</small></div></li>
      </ol>
      <div class="rail-note"><i class="ph ph-shield-check" aria-hidden="true"></i><p>음성과 결과는 이 Mac 안에서만 처리됩니다.</p></div>
    </aside>

    <main class="workspace">
      <header class="topbar">
        <div><span class="eyebrow">LOCAL AUDIO WORKSPACE</span><h1>회의에서 중요한 갈피를 찾으세요.</h1></div>
        <div class="engine-chip"><span id="setup-state" data-state="pending">확인 중</span><small id="engine-version">WhisperX</small></div>
      </header>

      <div class="workspace-body">
        <section id="setup-panel" class="panel setup-panel" aria-labelledby="setup-title">
          <div class="section-heading">
            <div><span class="section-index">01 / 준비</span><h2 id="setup-title">로컬 AI 환경</h2></div>
            <p>Galpi가 앱 전용 Python과 WhisperX를 설치하고 필요한 모델을 한 번에 준비합니다.</p>
          </div>
          <div class="setup-grid">
            <div class="status-list" aria-label="설치 상태">
              <div id="engine-check" class="status-row" data-state="pending"><i class="ph ph-cpu"></i><span data-status-label>WhisperX 엔진</span><strong data-status-value>확인 중</strong></div>
              <div id="model-check" class="status-row" data-state="pending"><i class="ph ph-brain"></i><span data-status-label>전사 모델</span><strong data-status-value>확인 중</strong></div>
              <div id="ffmpeg-check" class="status-row" data-state="pending"><i class="ph ph-waveform"></i><span data-status-label>내장 ffmpeg</span><strong data-status-value>확인 중</strong></div>
            </div>
            <div class="token-field">
              <label for="hf-token">Hugging Face 토큰 <span>선택</span></label>
              <input id="hf-token" type="password" autocomplete="off" placeholder="hf_..." aria-describedby="token-help" />
              <p id="token-help">화자분리 모델을 처음 내려받는 경우에만 필요합니다. 토큰은 준비 프로세스에 한 번 전달되고 저장되지 않습니다.</p>
              <div class="token-guide-anchor">
                <button id="token-guide-trigger" class="token-guide-trigger" type="button" aria-expanded="false" aria-controls="token-guide-popover">필요 권한과 발급 방법 <i class="ph ph-info"></i></button>
                <div id="token-guide-popover" class="token-guide-popover" role="dialog" aria-label="Hugging Face 토큰 발급 안내" hidden>
                  <div class="token-guide-header"><strong>토큰 발급 안내</strong><button id="token-guide-close" type="button" aria-label="닫기"><i class="ph ph-x"></i></button></div>
                  <p><strong>권장 권한:</strong> Fine-grained 토큰의 읽기(Read) 전용 권한만 사용하세요. 쓰기·추론 API 권한은 필요하지 않습니다.</p>
                  <ol>
                    <li>Hugging Face 계정에 로그인합니다.</li>
                    <li>아래 모델 페이지에서 이용 조건에 동의하고 접근 승인을 받습니다.</li>
                    <li>Settings → Access Tokens → Create new token에서 <strong>Fine-grained</strong>를 선택합니다.</li>
                    <li><code>pyannote/speaker-diarization-community-1</code> 저장소 콘텐츠의 Read 권한만 허용합니다.</li>
                    <li><code>hf_</code>로 시작하는 토큰을 복사해 위 입력란에 붙여넣습니다.</li>
                  </ol>
                  <p>접근 승인이 끝났거나 모델이 이미 이 Mac에 준비되어 있으면 토큰을 비워 두어도 됩니다.</p>
                </div>
              </div>
              <button class="text-button" type="button" data-action="model-access">모델 이용 조건 페이지 열기 <i class="ph ph-arrow-up-right"></i></button>
            </div>
          </div>
          <div id="setup-progress-panel" class="setup-progress-card" hidden>
            <div class="job-header"><div><span class="section-index">앱 전용 환경 설치</span><h3>로컬 환경 준비 중</h3></div><strong id="setup-job-percent">0%</strong></div>
            <div id="setup-job-progress" class="wave-progress" role="progressbar" aria-label="로컬 환경 준비 진행률" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"><span></span></div>
            <ol class="phase-list setup-phase-list">
              <li data-setup-phase="engine" data-state="pending">Python · WhisperX</li>
              <li data-setup-phase="models" data-state="pending">모델 · ffmpeg</li>
            </ol>
            <p id="setup-job-message" class="job-message" aria-live="polite"></p>
            <p id="setup-error-message" class="error-message" role="alert" hidden></p>
            <div class="job-actions">
              <button id="setup-cancel-button" class="secondary-button danger" type="button" data-action="cancel">설치 취소</button>
              <details><summary>설치 상세 로그</summary><pre id="setup-log-output"></pre></details>
            </div>
          </div>
          <div class="panel-actions">
            <button id="prepare-button" class="primary-button" type="button" data-action="prepare"><i class="ph ph-download-simple"></i><span>로컬 엔진 준비</span></button>
            <span class="action-note">첫 실행은 약 3GB의 모델을 내려받을 수 있습니다.</span>
          </div>
        </section>

        <section id="transcription-panel" class="panel transcription-panel" aria-labelledby="transcription-title">
          <div class="section-heading">
            <div><span class="section-index">02 / 전사</span><h2 id="transcription-title">새 회의 전사</h2></div>
            <p>참석 인원을 알려주면 겹치는 목소리와 짧은 발화를 더 안정적으로 분리합니다.</p>
          </div>
          <div class="transcription-grid">
            <div class="primary-task">
              <div id="recorder" class="recorder" data-state="idle">
                <button id="record-button" class="record-button" type="button" data-action="record">
                  <span><i class="ph ph-microphone"></i></span>
                  <div><strong>마이크로 바로 녹음</strong><small>CoreAudio · 16-bit PCM WAV</small></div>
                </button>
                <div id="recording-active" class="recording-active" hidden>
                  <span class="recording-dot" aria-hidden="true"></span>
                  <div><strong id="recording-label">녹음 중</strong><small id="recording-path">마이크 입력을 저장하고 있습니다.</small></div>
                  <time id="recording-time">00:00</time>
                  <button id="stop-recording-button" class="record-stop" type="button" data-action="stop-recording"><i class="ph ph-stop"></i> 정지</button>
                  <button id="cancel-recording-button" class="record-discard" type="button" data-action="cancel-recording">버리기</button>
                </div>
                <p id="recording-status" class="recording-status" aria-live="polite">마이크로 바로 녹음하거나 기존 파일을 선택하세요.</p>
              </div>
              <div class="choice-divider"><span>또는 기존 파일</span></div>
              <button id="audio-selection" class="file-picker" type="button" data-action="choose-audio" data-selected="false">
                <span class="file-icon"><i class="ph ph-music-notes"></i></span>
                <span><strong>오디오 파일 선택</strong><small id="audio-path">m4a, mp3, wav, mp4, mov, flac</small></span>
                <i class="ph ph-caret-right"></i>
              </button>
              <div class="field-block">
                <div class="field-label"><span>출력 폴더</span><button id="output-button" type="button" data-action="choose-output">변경</button></div>
                <div id="output-path" class="path-display">출력 폴더를 선택하세요.</div>
              </div>
            </div>
            <fieldset class="speaker-panel">
              <legend>화자 수 힌트</legend>
              <div class="segmented-control">
                <label><input type="radio" name="speaker-mode" value="auto" checked /><span>자동</span></label>
                <label><input type="radio" name="speaker-mode" value="exact" /><span>정확히</span></label>
                <label><input type="radio" name="speaker-mode" value="range" /><span>범위</span></label>
              </div>
              <p>참석 인원을 모르면 자동을 선택해도 됩니다.</p>
              <div id="exact-fields" class="number-fields" hidden><label for="exact-speakers">참석 인원</label><input id="exact-speakers" type="number" min="1" max="30" value="4" /></div>
              <div id="range-fields" class="number-fields range" hidden><label for="min-speakers">최소</label><input id="min-speakers" type="number" min="1" max="30" value="3" /><span>–</span><label for="max-speakers">최대</label><input id="max-speakers" type="number" min="1" max="30" value="7" /></div>
            </fieldset>
          </div>
          <div class="panel-actions">
            <button id="start-button" class="primary-button" type="button" data-action="transcribe" disabled><i class="ph ph-play"></i><span>전사 시작</span></button>
            <span class="action-note">체크포인트가 있으면 전사·정렬을 재사용합니다.</span>
          </div>
        </section>

        <section id="job-panel" class="panel job-panel" hidden aria-labelledby="job-title">
          <div class="job-header"><div><span id="busy-label" class="section-index"></span><h2 id="job-title">작업 진행</h2></div><strong id="job-percent">0%</strong></div>
          <div id="job-progress" class="wave-progress" role="progressbar" aria-label="현재 단계 진행률" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"><span></span></div>
          <ol class="phase-list">
            <li data-phase="transcribing" data-state="pending">전사</li>
            <li data-phase="aligning" data-state="pending">정렬</li>
            <li data-phase="diarizing" data-state="pending">화자분리</li>
            <li data-phase="writing" data-state="pending">결과 저장</li>
          </ol>
          <p id="job-message" class="job-message" aria-live="polite"></p>
          <p id="error-message" class="error-message" role="alert" hidden></p>
          <div class="job-actions">
            <button id="cancel-button" class="secondary-button danger" type="button" data-action="cancel" hidden>작업 취소</button>
            <details><summary>상세 로그</summary><pre id="log-output"></pre></details>
          </div>
        </section>

        <section id="results-panel" class="panel results-panel" hidden aria-labelledby="results-title">
          <div class="section-heading"><div><span class="section-index">03 / 완료</span><h2 id="results-title">전사 결과</h2></div><p id="result-summary"></p></div>
          <div class="artifact-list">
            <div class="artifact-row"><i class="ph ph-subtitles"></i><div><strong>자막 파일</strong><code id="result-srt"></code></div><button type="button" data-action="open-srt">열기</button></div>
            <div class="artifact-row"><i class="ph ph-users-three"></i><div><strong>화자별 텍스트</strong><code id="result-txt"></code></div><button type="button" data-action="open-txt">열기</button></div>
            <div class="artifact-row"><i class="ph ph-database"></i><div><strong>정렬 체크포인트</strong><code id="result-checkpoint"></code></div><button type="button" data-action="open-checkpoint">열기</button></div>
          </div>
          <div class="panel-actions"><button class="secondary-button" type="button" data-action="reveal-output"><i class="ph ph-folder-open"></i>Finder에서 보기</button></div>
        </section>
      </div>
      <footer class="app-footer"><span>Galpi 0.1</span><span>모든 추론은 로컬에서 실행됩니다.</span></footer>
    </main>
  </div>
`
