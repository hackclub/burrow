pub use burrowgrpc::*;

mod burrowgrpc {
    tonic::include_proto!("burrow");
}
