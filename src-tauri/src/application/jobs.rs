use crate::application::error::AppError;
use crate::domain::artifact::Artifacts;
use std::collections::HashMap;
use std::path::PathBuf;
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

/// Holds the single active-job slot for as long as the job runs.
///
/// Releasing on drop is what makes the slot safe: an early `?`, a cancelled
/// future, or an unwinding panic would otherwise leave the registry believing a
/// job is still running and every later request would be refused as `BUSY`.
pub struct JobGuard<'a> {
    registry: &'a JobRegistry,
    id: Uuid,
}

impl JobGuard<'_> {
    pub const fn id(&self) -> Uuid {
        self.id
    }
}

impl Drop for JobGuard<'_> {
    fn drop(&mut self) {
        // `finish` only fails on a poisoned lock, and a destructor has nowhere
        // to report that; the slot is released either way on the next claim.
        let _released = self.registry.finish(self.id);
    }
}

impl JobRegistry {
    pub fn claim_with_id(
        &self,
        id: Uuid,
    ) -> Result<(JobGuard<'_>, oneshot::Receiver<()>), AppError> {
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
        Ok((JobGuard { registry: self, id }, receiver))
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

    pub fn register_minutes(&self, id: Uuid, minutes: PathBuf) -> Result<(), AppError> {
        self.artifacts
            .lock()
            .map_err(|_| AppError::new("STATE_ERROR", "결과 상태 잠금이 손상되었습니다."))?
            .get_mut(&id)
            .ok_or_else(|| AppError::new("ARTIFACT_NOT_FOUND", "완료된 결과를 찾지 못했습니다."))?
            .minutes = Some(minutes);
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

#[cfg(test)]
mod tests {
    use super::JobRegistry;
    use uuid::Uuid;

    #[test]
    fn only_one_job_runs_at_a_time() {
        // Given: a job holding the slot
        let registry = JobRegistry::default();
        let claimed = registry.claim_with_id(Uuid::now_v7());
        assert!(claimed.is_ok());

        // When
        let refused = registry.claim_with_id(Uuid::now_v7());

        // Then
        assert_eq!(
            refused.err().map(|error| error.code),
            Some("BUSY".to_owned())
        );
    }

    #[test]
    fn dropping_the_guard_frees_the_slot_without_an_explicit_finish() {
        // Given: a job that ended without anyone calling finish
        let registry = JobRegistry::default();
        {
            let claimed = registry.claim_with_id(Uuid::now_v7());
            assert!(claimed.is_ok());
        }

        // When / Then: the next job is not refused as BUSY
        assert!(registry.claim_with_id(Uuid::now_v7()).is_ok());
    }

    #[test]
    fn cancelling_twice_reports_that_it_is_already_stopping() {
        // Given
        let registry = JobRegistry::default();
        let id = Uuid::now_v7();
        let claimed = registry.claim_with_id(id);
        assert!(claimed.is_ok());

        // When
        assert!(registry.cancel(id).is_ok());
        let again = registry.cancel(id);

        // Then
        assert_eq!(
            again.err().map(|error| error.code),
            Some("ALREADY_CANCELLING".to_owned())
        );
    }

    #[test]
    fn cancelling_an_unknown_job_is_reported_as_such() {
        // Given
        let registry = JobRegistry::default();
        let claimed = registry.claim_with_id(Uuid::now_v7());
        assert!(claimed.is_ok());

        // When / Then
        assert_eq!(
            registry.cancel(Uuid::now_v7()).err().map(|e| e.code),
            Some("JOB_NOT_FOUND".to_owned())
        );
    }
}
