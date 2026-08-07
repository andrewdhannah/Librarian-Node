pub mod capabilities;
pub mod hardware;
pub mod identity;
pub mod registration;
pub mod startup;
pub mod state;

pub use capabilities::{Capability, CapabilityManifest, ModelDescriptor};
pub use hardware::HardwareProfile;
pub use identity::{NodeIdentity, NodeStatus};
pub use registration::{NodeRecord, RegistrationReceipt, RegistrationRequest};
pub use startup::{
    RuntimeLifecycleState, StartupCheck, StartupPhase, StartupReceipt, StartupReceiptFacts, StartupStatus,
};
pub use state::NodeState;
