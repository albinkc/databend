// Copyright 2021 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use async_trait::async_trait;
use common_meta_kvapi::kvapi;
use common_meta_types::AppliedState;
use common_meta_types::Cmd;
use common_meta_types::GetKVReply;
use common_meta_types::GetKVReq;
use common_meta_types::KVAppError;
use common_meta_types::ListKVReply;
use common_meta_types::ListKVReq;
use common_meta_types::LogEntry;
use common_meta_types::MGetKVReply;
use common_meta_types::MGetKVReq;
use common_meta_types::TxnReply;
use common_meta_types::TxnRequest;
use common_meta_types::UpsertKV;
use common_meta_types::UpsertKVReply;
use common_meta_types::UpsertKVReq;
use tracing::info;

use crate::meta_service::MetaNode;

/// Impl kvapi::KVApi for MetaNode.
///
/// Write through raft-log.
/// Read through local state machine, which may not be consistent.
/// E.g. Read is not guaranteed to see a write.
#[async_trait]
impl kvapi::KVApi for MetaNode {
    type Error = KVAppError;

    async fn upsert_kv(&self, act: UpsertKVReq) -> Result<UpsertKVReply, KVAppError> {
        let ent = LogEntry {
            txid: None,
            time_ms: None,
            cmd: Cmd::UpsertKV(UpsertKV {
                key: act.key,
                seq: act.seq,
                value: act.value,
                value_meta: act.value_meta,
            }),
        };
        let rst = self.write(ent).await?;

        match rst {
            AppliedState::KV(x) => Ok(x),
            _ => {
                unreachable!("expect type {}", "AppliedState::KV")
            }
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn get_kv(&self, key: &str) -> Result<GetKVReply, KVAppError> {
        let res = self
            .consistent_read(GetKVReq {
                key: key.to_string(),
            })
            .await?;

        Ok(res)
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn mget_kv(&self, keys: &[String]) -> Result<MGetKVReply, KVAppError> {
        let res = self
            .consistent_read(MGetKVReq {
                keys: keys.to_vec(),
            })
            .await?;

        Ok(res)
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn prefix_list_kv(&self, prefix: &str) -> Result<ListKVReply, KVAppError> {
        let res = self
            .consistent_read(ListKVReq {
                prefix: prefix.to_string(),
            })
            .await?;

        Ok(res)
    }

    #[tracing::instrument(level = "debug", skip(self, txn))]
    async fn transaction(&self, txn: TxnRequest) -> Result<TxnReply, KVAppError> {
        info!("MetaNode::transaction(): {}", txn);
        let ent = LogEntry {
            txid: None,
            time_ms: None,
            cmd: Cmd::Transaction(txn),
        };
        let rst = self.write(ent).await?;

        match rst {
            AppliedState::TxnReply(x) => Ok(x),
            _ => {
                unreachable!("expect type {}", "AppliedState::transaction",)
            }
        }
    }
}
