pub use network_custom_rpc_runtime_api::NetworkRuntimeApi;
use jsonrpsee::{
	core::{Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Custom {
	code: u32,
	sum: u32,
}

#[rpc(client, server)]
pub trait NetworkCustomApi<BlockHash> {
	#[method(name = "network_getModelPeers")]
	fn get_model_peers(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getModelPeersInclude")]
	fn get_model_peers_include(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getModelPeersSubmittable")]
	fn get_model_peers_submittable(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getModelPeersModelUnconfirmedCount")]
	fn get_model_peers_model_unconfirmed_count(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<u32>;
}

/// A struct that implements the `NetworkCustomApi`.
pub struct NetworkCustom<C, Block> {
	// If you have more generics, no need to NetworkCustom<C, M, N, P, ...>
	// just use a tuple like NetworkCustom<C, (M, N, P, ...)>
	client: Arc<C>,
	_marker: std::marker::PhantomData<Block>,
}

impl<C, Block> NetworkCustom<C, Block> {
	/// Create new `NetworkCustom` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { 
      client, 
      _marker: Default::default() 
    }
	}
}

/// Error type of this RPC api.
pub enum Error {
  /// The call to runtime failed.
  RuntimeError,
}

impl From<Error> for i32 {
  fn from(e: Error) -> i32 {
      match e {
          Error::RuntimeError => 1,
      }
  }
}

impl<C, Block> NetworkCustomApiServer<<Block as BlockT>::Hash> for NetworkCustom<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: NetworkRuntimeApi<Block>,
{
	fn get_model_peers(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_model_peers(at, model_id).map_err(runtime_error_into_rpc_err)
	}
	fn get_model_peers_include(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_model_peers_included(at, model_id).map_err(runtime_error_into_rpc_err)
	}
	fn get_model_peers_submittable(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_model_peers_submittable(at, model_id).map_err(runtime_error_into_rpc_err)
	}
	fn get_model_peers_model_unconfirmed_count(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<u32> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_model_peers_model_unconfirmed_count(at, model_id).map_err(runtime_error_into_rpc_err)
	}
}

const RUNTIME_ERROR: i32 = 1;

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
  CallError::Custom(ErrorObject::owned(
    Error::RuntimeError.into(),
    "Runtime error",
    Some(format!("{:?}", err)),
  ))
  .into()
}
