//! Evaluation-only tripwire provider.
//!
//! This module exists to answer one question the release cannot answer about
//! itself: when the VM reaches its mediation point and invokes a provider,
//! what does it hand over, and would an egress attempt from that position be
//! observed?
//!
//! It is compiled only under the `eval-tripwire` feature, which the release
//! build never enables. The provider occupies the built-in `simulated` slot
//! and behaves exactly like [`crate::SimulatedProvider`] unless an egress
//! probe is configured; the probe exists so the campaign can demonstrate a
//! positive control *at the mediation point* rather than only from the
//! harness. No production code path can reach any of this.

use crate::{
    ModelProviderRequest, ModelProviderResponse, Provider, ProviderCallStatus, ProviderError,
    ProviderKind, ToolProviderRequest, ToolProviderResponse,
};
use std::{
    io::Write,
    net::TcpStream,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

/// What the tripwire observed, shared with whoever registered it.
#[derive(Debug, Default)]
pub struct TripwireLog {
    invocations: AtomicUsize,
    tool_invocations: AtomicUsize,
    model_invocations: AtomicUsize,
    non_dry_run_requests: AtomicUsize,
    egress_attempted: AtomicBool,
    egress_succeeded: AtomicBool,
}

impl TripwireLog {
    pub fn invocations(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
    }

    pub fn tool_invocations(&self) -> usize {
        self.tool_invocations.load(Ordering::SeqCst)
    }

    pub fn model_invocations(&self) -> usize {
        self.model_invocations.load(Ordering::SeqCst)
    }

    /// Requests the VM handed over without the dry-run flag set.
    pub fn non_dry_run_requests(&self) -> usize {
        self.non_dry_run_requests.load(Ordering::SeqCst)
    }

    pub fn egress_attempted(&self) -> bool {
        self.egress_attempted.load(Ordering::SeqCst)
    }

    pub fn egress_succeeded(&self) -> bool {
        self.egress_succeeded.load(Ordering::SeqCst)
    }
}

/// A stand-in for the built-in simulated provider that records what the VM
/// does at the mediation point, and can optionally probe a loopback sink.
pub struct TripwireProvider {
    log: Arc<TripwireLog>,
    /// `host:port/path` of a loopback sink to probe on every invocation.
    egress_target: Option<String>,
}

impl TripwireProvider {
    pub fn new(log: Arc<TripwireLog>) -> Self {
        Self {
            log,
            egress_target: None,
        }
    }

    /// Probe the given target on every invocation. Used as a positive control:
    /// if the campaign's sink records this hit, egress from the mediation
    /// point is observable, so zero hits elsewhere is a measurement.
    pub fn with_egress_probe(mut self, target: impl Into<String>) -> Self {
        self.egress_target = Some(target.into());
        self
    }

    fn record(&self, dry_run: bool) {
        self.log.invocations.fetch_add(1, Ordering::SeqCst);
        if !dry_run {
            self.log.non_dry_run_requests.fetch_add(1, Ordering::SeqCst);
        }
        self.probe();
    }

    fn probe(&self) {
        let Some(target) = &self.egress_target else {
            return;
        };
        self.log.egress_attempted.store(true, Ordering::SeqCst);
        let (authority, path) = match target.split_once('/') {
            Some((authority, rest)) => (authority, format!("/{rest}")),
            None => (target.as_str(), "/".to_owned()),
        };
        let Ok(mut stream) = TcpStream::connect(authority) else {
            return;
        };
        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
        let request = format!(
            "GET {path} HTTP/1.1\r\nHost: {authority}\r\nUser-Agent: argorix-eval-tripwire\r\nConnection: close\r\n\r\n"
        );
        if stream.write_all(request.as_bytes()).is_ok() {
            self.log.egress_succeeded.store(true, Ordering::SeqCst);
        }
    }
}

impl Provider for TripwireProvider {
    fn name(&self) -> &str {
        "simulated"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Simulated
    }

    fn invoke_model(
        &self,
        request: ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ProviderError> {
        self.log.model_invocations.fetch_add(1, Ordering::SeqCst);
        self.record(request.dry_run);
        if !request.dry_run {
            return Err(ProviderError::DryRunRequired);
        }
        Ok(ModelProviderResponse {
            call_id: request.call_id,
            status: ProviderCallStatus::Allowed,
            output_type: request.output_type,
            simulated: true,
        })
    }

    fn invoke_tool(
        &self,
        request: ToolProviderRequest,
    ) -> Result<ToolProviderResponse, ProviderError> {
        self.log.tool_invocations.fetch_add(1, Ordering::SeqCst);
        self.record(request.dry_run);
        if !request.dry_run {
            return Err(ProviderError::DryRunRequired);
        }
        Ok(ToolProviderResponse {
            call_id: request.call_id,
            status: ProviderCallStatus::Allowed,
            output_type: request.output_type,
            simulated: true,
        })
    }
}
