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

use std::slice::Iter;

use common_datavalues::Column;
use common_datavalues::LargePrimitive;
use common_datavalues::PrimitiveColumn;
use common_datavalues::PrimitiveType;
use common_datavalues::ScalarColumn;
use common_datavalues::StringColumn;
use common_exception::Result;

pub trait KeysColumnIter<T: ?Sized> {
    type Iterator<'a>: Iterator<Item = &'a T>
    where
        Self: 'a,
        T: 'a;

    fn iter(&self) -> Self::Iterator<'_>;
}

pub struct FixedKeysColumnIter<T: PrimitiveType> {
    column: PrimitiveColumn<T>,
}

impl<T: PrimitiveType> FixedKeysColumnIter<T> {
    pub fn create(column: &PrimitiveColumn<T>) -> Result<Self> {
        Ok(Self {
            column: column.clone(),
        })
    }
}

impl<T: PrimitiveType> KeysColumnIter<T> for FixedKeysColumnIter<T> {
    type Iterator<'a> = Iter<'a, T> where Self: 'a, T: 'a;

    fn iter(&self) -> Self::Iterator<'_> {
        self.column.values().into_iter()
    }
}

pub struct LargeFixedKeysColumnIter<T: LargePrimitive> {
    holder: Vec<T>,
}

impl<T: LargePrimitive> LargeFixedKeysColumnIter<T> {
    pub fn create(inner: &StringColumn) -> Result<Self> {
        let mut array = Vec::with_capacity(inner.len());

        for bs in inner.scalar_iter() {
            array.push(T::from_bytes(bs)?);
        }

        Ok(Self { holder: array })
    }
}

impl<T: LargePrimitive> KeysColumnIter<T> for LargeFixedKeysColumnIter<T> {
    type Iterator<'a> = Iter<'a, T> where Self: 'a, T: 'a;

    fn iter(&self) -> Self::Iterator<'_> {
        self.holder.iter()
    }
}

pub struct SerializedKeysColumnIter {
    column: StringColumn,
}

impl SerializedKeysColumnIter {
    pub fn create(column: &StringColumn) -> Result<SerializedKeysColumnIter> {
        Ok(SerializedKeysColumnIter {
            column: column.clone(),
        })
    }
}

pub struct SerializedKeysIter<'a> {
    data: &'a [u8],
    offsets: &'a [i64],
    pos: usize,
}

impl<'a> Iterator for SerializedKeysIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < (self.offsets.len() - 1) {
            let offset = self.offsets[self.pos] as usize;
            let offset_1 = self.offsets[self.pos + 1] as usize;
            return Some(&self.data[offset..offset_1]);
        }

        None
    }
}

impl KeysColumnIter<[u8]> for SerializedKeysColumnIter {
    type Iterator<'a> = SerializedKeysIter<'a> where Self: 'a;

    fn iter(&self) -> Self::Iterator<'_> {
        SerializedKeysIter {
            pos: 0,
            data: self.column.values(),
            offsets: self.column.offsets(),
        }
    }
}
