// Copyright 2022 Datafuse Labs.
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

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use common_exception::ErrorCode;
use common_exception::Result;
use common_expression::Chunk;
use common_expression::ChunkCompactThresholds;

use super::Compactor;
use super::TransformCompact;

pub struct ChunkCompactorNoSplit {
    thresholds: ChunkCompactThresholds,
    aborting: Arc<AtomicBool>,
    // call chunk.memory_size() only once.
    // we may no longer need it if we start using jsonb, otherwise it should be put in CompactorState
    accumulated_rows: usize,
    accumulated_bytes: usize,
}

impl ChunkCompactorNoSplit {
    pub fn new(thresholds: ChunkCompactThresholds) -> Self {
        ChunkCompactorNoSplit {
            thresholds,
            accumulated_rows: 0,
            accumulated_bytes: 0,
            aborting: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Compactor for ChunkCompactorNoSplit {
    fn name() -> &'static str {
        "ChunkCompactTransform"
    }

    fn use_partial_compact() -> bool {
        true
    }

    fn interrupt(&self) {
        self.aborting.store(true, Ordering::Release);
    }

    fn compact_partial(&mut self, chunks: &mut Vec<Chunk>) -> Result<Vec<Chunk>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        let size = chunks.len();
        let mut res = Vec::with_capacity(size);
        let chunk = chunks[size - 1].clone();

        let num_rows = chunk.num_rows();
        let num_bytes = chunk.memory_size();

        if self.thresholds.check_large_enough(num_rows, num_bytes) {
            // pass through the new data chunk just arrived
            res.push(chunk);
            chunks.remove(size - 1);
        } else {
            let accumulated_rows_new = self.accumulated_rows + num_rows;
            let accumulated_bytes_new = self.accumulated_bytes + num_bytes;

            if self
                .thresholds
                .check_large_enough(accumulated_rows_new, accumulated_bytes_new)
            {
                // avoid call concat_chunks for each new chunk
                let merged = Chunk::concat(chunks)?;
                chunks.clear();
                self.accumulated_rows = 0;
                self.accumulated_bytes = 0;
                res.push(merged);
            } else {
                self.accumulated_rows = accumulated_rows_new;
                self.accumulated_bytes = accumulated_bytes_new;
            }
        }

        Ok(res)
    }

    fn compact_final(&self, chunks: &[Chunk]) -> Result<Vec<Chunk>> {
        let mut res = vec![];
        if self.accumulated_rows != 0 {
            if self.aborting.load(Ordering::Relaxed) {
                return Err(ErrorCode::AbortedQuery(
                    "Aborted query, because the server is shutting down or the query was killed.",
                ));
            }

            let chunk = Chunk::concat(chunks)?;
            res.push(chunk);
        }

        Ok(res)
    }
}

pub type TransformChunkCompact = TransformCompact<ChunkCompactorNoSplit>;