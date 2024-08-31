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
	#[method(name = "network_getSubnetNodes")]
	fn get_subnet_nodes(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getSubnetNodesIncluded")]
	fn get_subnet_nodes_included(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getSubnetNodesSubmittable")]
	fn get_subnet_nodes_submittable(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getSubnetNodesUnconfirmedCount")]
	fn get_subnet_nodes_model_unconfirmed_count(&self, model_id: u32, at: Option<BlockHash>) -> RpcResult<u32>;
	#[method(name = "network_getConsensusData")]
	fn get_consensus_data(&self, model_id: u32, epoch: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getAccountantData")]
	fn get_accountant_data(&self, model_id: u32, id: u32, at: Option<BlockHash>) -> RpcResult<Vec<u8>>;
	#[method(name = "network_getMinimumSubnetNodes")]
	fn get_minimum_subnet_nodes(&self, subnet_id: u32, memory_mb: u128, at: Option<BlockHash>) -> RpcResult<u32>;

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
	fn get_subnet_nodes(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_subnet_nodes(at, model_id).map_err(runtime_error_into_rpc_err)
	}
	fn get_subnet_nodes_included(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_subnet_nodes_included(at, model_id).map_err(runtime_error_into_rpc_err)
	}
	fn get_subnet_nodes_submittable(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_subnet_nodes_submittable(at, model_id).map_err(runtime_error_into_rpc_err)
	}
	fn get_subnet_nodes_model_unconfirmed_count(&self, model_id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<u32> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_subnet_nodes_model_unconfirmed_count(at, model_id).map_err(runtime_error_into_rpc_err)
	}
	fn get_consensus_data(&self, model_id: u32, epoch: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_consensus_data(at, model_id, epoch).map_err(runtime_error_into_rpc_err)
	}
	fn get_accountant_data(&self, model_id: u32, id: u32, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<u8>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_accountant_data(at, model_id, id).map_err(runtime_error_into_rpc_err)
	}
	fn get_minimum_subnet_nodes(&self, subnet_id: u32, memory_mb: u128, at: Option<<Block as BlockT>::Hash>) -> RpcResult<u32> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		api.get_minimum_subnet_nodes(at, subnet_id, memory_mb).map_err(runtime_error_into_rpc_err)
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
