use crate::application::error::AppError;
use crate::domain::artifact::Artifacts;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug)]
struct ActiveJob {
    id: Uuid,
    cancel: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Default)]
pub struct JobRegistry {
    active: Mutex<Option<ActiveJob>>,
    artifacts: Mutex<HashMap<Uuid, Artifacts>>,
}

impl JobRegistry {
    pub fn claim(&self) -> Result<(Uuid, oneshot::Receiver<()>), AppError> {
        self.claim_with_id(Uuid::now_v7())
    }

    pub fn claim_with_id(&self, id: Uuid) -> Result<(Uuid, oneshot::Receiver<()>), AppError> {
        let mut active = self.active_lock()?;
        if active.is_some() {
            return Err(AppError::new("BUSY", "이미 실행 중인 작업이 있습니다."));
        }
        if self
            .artifacts
            .lock()
            .map_err(|_| AppError::new("STATE_ERROR", "결과 상태 잠금이 손상되었습니다."))?
            .contains_key(&id)
        {
            return Err(AppError::new(
                "JOB_ID_CONFLICT",
                "이미 사용된 작업 ID입니다.",
            ));
        }
        let (cancel, receiver) = oneshot::channel();
        *active = Some(ActiveJob {
            id,
            cancel: Some(cancel),
        });
        Ok((id, receiver))
    }

    pub fn finish(&self, id: Uuid) -> Result<(), AppError> {
        let mut active = self.active_lock()?;
        if active.as_ref().is_some_and(|job| job.id == id) {
            *active = None;
        }
        Ok(())
    }

    pub fn cancel(&self, id: Uuid) -> Result<(), AppError> {
        let mut active = self.active_lock()?;
        let job = active
            .as_mut()
            .filter(|job| job.id == id)
            .ok_or_else(|| AppError::new("JOB_NOT_FOUND", "취소할 작업을 찾지 못했습니다."))?;
        let cancel = job
            .cancel
            .take()
            .ok_or_else(|| AppError::new("ALREADY_CANCELLING", "이미 취소 중입니다."))?;
        cancel
            .send(())
            .map_err(|()| AppError::new("JOB_FINISHED", "작업이 이미 종료되었습니다."))
    }

    pub fn register(&self, id: Uuid, artifacts: Artifacts) -> Result<(), AppError> {
        self.artifacts
            .lock()
            .map_err(|_| AppError::new("STATE_ERROR", "결과 상태 잠금이 손상되었습니다."))?
            .insert(id, artifacts);
        Ok(())
    }

    pub fn artifacts(&self, id: Uuid) -> Result<Artifacts, AppError> {
        self.artifacts
            .lock()
            .map_err(|_| AppError::new("STATE_ERROR", "결과 상태 잠금이 손상되었습니다."))?
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::new("ARTIFACT_NOT_FOUND", "완료된 결과를 찾지 못했습니다."))
    }

    fn active_lock(&self) -> Result<std::sync::MutexGuard<'_, Option<ActiveJob>>, AppError> {
        self.active
            .lock()
            .map_err(|_| AppError::new("STATE_ERROR", "작업 상태 잠금이 손상되었습니다."))
    }
}
