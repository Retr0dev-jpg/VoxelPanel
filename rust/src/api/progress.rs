// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

use std::future::Future;

use crate::frb_generated::StreamSink;
use crate::{PanelResult, ProgressTx};

use super::types::ProgressEvent;

/// Runs a long operation and streams its progress, then a final done or error event.
pub(crate) async fn report<T, F, Fut>(
    sink: StreamSink<ProgressEvent>,
    done_stage: &str,
    done_message: impl FnOnce(&T) -> (String, Option<String>),
    operation: F,
) -> PanelResult<()>
where
    F: FnOnce(ProgressTx) -> Fut,
    Fut: Future<Output = PanelResult<T>>,
{
    let forward = sink.clone();
    let tx = ProgressTx::new(move |progress| {
        let _ = forward.add(ProgressEvent {
            stage: progress.stage,
            message: progress.message,
            fraction: progress.fraction,
            done: false,
            error: None,
            server_id: None,
        });
    });
    match operation(tx).await {
        Ok(value) => {
            let (message, server_id) = done_message(&value);
            tracing::info!(stage = done_stage, "{message}");
            let _ = sink.add(ProgressEvent {
                stage: done_stage.to_string(),
                message,
                fraction: Some(1.0),
                done: true,
                error: None,
                server_id,
            });
            Ok(())
        }
        Err(error) => {
            tracing::error!(stage = done_stage, "{}", error.message);
            let _ = sink.add(ProgressEvent {
                stage: "Errore".into(),
                message: error.message.clone(),
                fraction: None,
                done: true,
                error: Some(error.message.clone()),
                server_id: None,
            });
            Err(error)
        }
    }
}
