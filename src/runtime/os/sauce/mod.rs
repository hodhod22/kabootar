//! Competitive "secret sauce" — 9 non-technical OS strategies (sandbox model).

mod ai_composer;
mod compat;
mod energy;
mod haptic;
mod privacy;
mod seamless;
mod setup;
mod state_sep;
mod updates;

pub use ai_composer::AiComposer;
pub use compat::{CompatEngine, CompatPlatform};
pub use energy::EnergyCore;
pub use haptic::HapticUi;
pub use privacy::PrivacyCore;
pub use seamless::SeamlessEcosystem;
pub use setup::ZeroTouchSetup;
pub use state_sep::StateSeparation;
pub use updates::{CommunityUpdates, UpdateChannel};

pub struct SauceSubsystem {
    pub ai: AiComposer,
    pub setup: ZeroTouchSetup,
    pub state: StateSeparation,
    pub seamless: SeamlessEcosystem,
    pub energy: EnergyCore,
    pub haptic: HapticUi,
    pub compat: CompatEngine,
    pub privacy: PrivacyCore,
    pub updates: CommunityUpdates,
}

impl Default for SauceSubsystem {
    fn default() -> Self {
        Self {
            ai: AiComposer::default(),
            setup: ZeroTouchSetup::default(),
            state: StateSeparation::default(),
            seamless: SeamlessEcosystem::default(),
            energy: EnergyCore::default(),
            haptic: HapticUi::default(),
            compat: CompatEngine::default(),
            privacy: PrivacyCore::default(),
            updates: CommunityUpdates::default(),
        }
    }
}

impl SauceSubsystem {
    pub fn strategy_map(&self) -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert(
            "s1_ai_prefetch".into(),
            self.ai.prefetch_targets().join(","),
        );
        m.insert(
            "s2_setup_secs".into(),
            format!("{}/{}", self.setup.elapsed_secs(), self.setup.target_secs()),
        );
        m.insert(
            "s3_partitions".into(),
            format!(
                "os+apps:{}+data:{}",
                self.state.partition_stats().1,
                self.state.partition_stats().2
            ),
        );
        m.insert(
            "s4_paired".into(),
            self.seamless.paired_count().to_string(),
        );
        let (bat, app, def, q) = self.energy.stats();
        m.insert(
            "s5_energy".into(),
            format!("battery={bat} active={app} deferred={def} queue={q}"),
        );
        m.insert("s6_haptic".into(), self.haptic.event_count().to_string());
        let (tr, hit, perf) = self.compat.stats();
        m.insert(
            "s7_compat".into(),
            format!("tr={tr} hit={hit} perf={perf}"),
        );
        m.insert(
            "s8_privacy".into(),
            format!("ram_lock={}", self.privacy.ram_locked()),
        );
        m.insert(
            "s9_updates".into(),
            format!(
                "{}@{}ms",
                self.updates.active_version(),
                self.updates.partition_swap_ms()
            ),
        );
        m
    }
}
