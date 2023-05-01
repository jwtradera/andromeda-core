pub mod ado_type;
pub mod block_height;
pub mod hooks;
pub mod kernel_address;
pub mod modules;
pub mod operators;
pub mod ownership;
#[cfg(feature = "primitive")]
pub mod primitive;
pub mod version;
#[cfg(feature = "withdraw")]
pub mod withdraw;
#[cfg(feature = "withdraw")]
use crate::ado_base::withdraw::Withdrawal;
#[cfg(feature = "withdraw")]
use crate::amp::recipient::Recipient;
use crate::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_binary, Binary, QuerierWrapper, QueryRequest, WasmQuery};
#[cfg(feature = "modules")]
pub use modules::Module;

#[cfg(feature = "modules")]
use cosmwasm_std::Uint64;

use serde::de::DeserializeOwned;

use self::{
    ado_type::TypeResponse,
    block_height::BlockHeightResponse,
    kernel_address::KernelAddressResponse,
    operators::{IsOperatorResponse, OperatorsResponse},
    ownership::{ContractOwnerResponse, PublisherResponse},
    version::VersionResponse,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub ado_type: String,
    pub ado_version: String,
    pub operators: Option<Vec<String>>,
    pub kernel_address: String,
    pub owner: Option<String>,
}

#[cw_serde]
pub enum AndromedaMsg {
    /// Standard Messages
    Receive(Option<Binary>),
    UpdateOwner {
        address: String,
    },
    UpdateOperators {
        operators: Vec<String>,
    },
    UpdateAppContract {
        address: String,
    },
    #[cfg(feature = "withdraw")]
    Withdraw {
        recipient: Option<Recipient>,
        tokens_to_withdraw: Option<Vec<Withdrawal>>,
    },
    #[cfg(feature = "modules")]
    RegisterModule {
        module: Module,
    },
    #[cfg(feature = "modules")]
    DeregisterModule {
        module_idx: Uint64,
    },
    #[cfg(feature = "modules")]
    AlterModule {
        module_idx: Uint64,
        module: Module,
    },
    #[cfg(feature = "primitive")]
    RefreshAddress {
        contract: String,
    },
    #[cfg(feature = "primitive")]
    RefreshAddresses {
        limit: Option<u32>,
        start_after: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum AndromedaQuery {
    #[returns(Option<Binary>)]
    Get(Option<Binary>),
    #[returns(ContractOwnerResponse)]
    Owner {},
    #[returns(OperatorsResponse)]
    Operators {},
    #[returns(TypeResponse)]
    Type {},
    #[returns(KernelAddressResponse)]
    KernelAddress {},
    #[returns(PublisherResponse)]
    OriginalPublisher {},
    #[returns(BlockHeightResponse)]
    BlockHeightUponCreation {},
    #[returns(IsOperatorResponse)]
    IsOperator { address: String },
    #[cfg(feature = "modules")]
    #[returns(Module)]
    Module { id: Uint64 },
    #[returns(Vec<String>)]
    ModuleIds {},
    #[returns(VersionResponse)]
    Version {},
}

/// Helper enum for serialization
#[cw_serde]
pub enum ExecuteMsg {
    AndrReceive(AndromedaMsg),
}

/// Helper enum for serialization
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AndromedaQuery)]
    AndrQuery(AndromedaQuery),
}

/// Helper function for querying a contract using AndromedaQuery::Get
pub fn query_get<T>(
    data: Option<Binary>,
    address: String,
    querier: &QuerierWrapper,
) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    let query_msg = QueryMsg::AndrQuery(AndromedaQuery::Get(data));
    let resp: T = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: address,
        msg: to_binary(&query_msg)?,
    }))?;

    Ok(resp)
}
