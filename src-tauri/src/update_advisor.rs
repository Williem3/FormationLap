use crate::{
    DesktopSettings, FormationLapInstallDecision, SessionState, UpdateCheckDecision,
    UpdateCheckPlan, UpdateCheckResult, UpdateCheckTrigger, UpdateSnapshot, UpdateStatus,
};

const AUTOMATIC_CHECK_INTERVAL_SECONDS: u64 = 86_400;

pub(crate) struct UpdateAdvisor {
    visible: UpdateSnapshot,
    pending_request_id: Option<String>,
    deferred_result: Option<UpdateCheckResult>,
}

impl UpdateAdvisor {
    pub(crate) fn new(last_automatic_check_unix_seconds: Option<u64>) -> Self {
        Self {
            visible: UpdateSnapshot {
                last_automatic_check_unix_seconds,
                ..UpdateSnapshot::default()
            },
            pending_request_id: None,
            deferred_result: None,
        }
    }

    pub(crate) fn snapshot(
        &self,
        last_automatic_check_unix_seconds: Option<u64>,
    ) -> UpdateSnapshot {
        let mut snapshot = self.visible.clone();
        snapshot.last_automatic_check_unix_seconds = last_automatic_check_unix_seconds;
        snapshot.result_deferred = self.deferred_result.is_some();
        snapshot
    }

    pub(crate) fn prepare_check(
        &mut self,
        trigger: UpdateCheckTrigger,
        now_unix_seconds: u64,
        session_state: &SessionState,
        settings: &DesktopSettings,
        last_automatic_check_unix_seconds: Option<u64>,
    ) -> UpdateCheckDecision {
        if *session_state != SessionState::Idle {
            return UpdateCheckDecision::Deferred;
        }
        if self.pending_request_id.is_some() {
            return UpdateCheckDecision::InProgress;
        }
        if trigger == UpdateCheckTrigger::Automatic {
            if !settings.automatic_update_checks {
                return UpdateCheckDecision::Disabled;
            }
            if last_automatic_check_unix_seconds.is_some_and(|last_check| {
                now_unix_seconds.saturating_sub(last_check) < AUTOMATIC_CHECK_INTERVAL_SECONDS
            }) {
                return UpdateCheckDecision::NotDue;
            }
        }

        let request_id = uuid::Uuid::new_v4().to_string();
        self.pending_request_id = Some(request_id.clone());
        UpdateCheckDecision::Planned(UpdateCheckPlan {
            request_id,
            channel: settings.update_channel.clone(),
            trigger,
            applications: Vec::new(),
        })
    }

    pub(crate) fn complete_check(
        &mut self,
        result: UpdateCheckResult,
        session_state: &SessionState,
    ) -> Result<(), String> {
        if self.pending_request_id.as_deref() != Some(result.request_id.as_str()) {
            return Err("update-check result does not match the pending request".to_owned());
        }
        self.pending_request_id = None;
        if *session_state == SessionState::Idle {
            self.apply_result(result);
        } else {
            self.deferred_result = Some(result);
        }
        Ok(())
    }

    pub(crate) fn cancel_check(&mut self, request_id: &str) -> bool {
        if self.pending_request_id.as_deref() == Some(request_id) {
            self.pending_request_id = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn release_deferred_if_safe(&mut self, session_state: &SessionState) {
        if *session_state == SessionState::Idle
            && let Some(result) = self.deferred_result.take()
        {
            self.apply_result(result);
        }
    }

    pub(crate) fn prepare_formation_lap_install(
        &self,
        session_state: &SessionState,
    ) -> FormationLapInstallDecision {
        if *session_state != SessionState::Idle {
            return FormationLapInstallDecision::Deferred;
        }
        match &self.visible.formation_lap {
            UpdateStatus::UpdateAvailable { latest_version, .. } => {
                FormationLapInstallDecision::Ready {
                    latest_version: latest_version.clone(),
                }
            }
            UpdateStatus::Current { .. } | UpdateStatus::Unknown { .. } => {
                FormationLapInstallDecision::NoUpdate
            }
        }
    }

    fn apply_result(&mut self, result: UpdateCheckResult) {
        self.visible.formation_lap = result.formation_lap;
        self.visible.applications = result.applications;
        self.visible.result_deferred = false;
    }
}
