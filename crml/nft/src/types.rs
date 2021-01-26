/* Copyright 2019-2021 Centrality Investments Limited
*
* Licensed under the LGPL, Version 3.0 (the "License");
* you may not use this file except in compliance with the License.
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific language governing permissions and
* limitations under the License.
* You may obtain a copy of the License at the root of this project source code,
* or at:
*     https://centrality.ai/licenses/gplv3.txt
*     https://centrality.ai/licenses/lgplv3.txt
*/

//! NFT module types

use codec::{Decode, Encode};
use sp_std::prelude::*;

/// Type Id of an NFTField
/// TODO: can't encode `u8`s
type NFTFieldTypeId = u8;
/// Some descriptive tag about an NFT field
type NFTFieldTag = [u8; 32];

/// Describes the data structure of an NFT class
pub type NFTSchema = Vec<NFTFieldTypeId>;

/// The supported data types for an NFT
#[derive(Decode, Encode, Debug, Clone, PartialEq)]
pub enum NFTField {
	I32(i32),
	U8(u8),
	U16(u16),
	U32(u32),
	U64(u64),
	U128(u128),
	Bytes32([u8; 32]),
}

impl NFTField {
	/// Return the type ID of this field
	pub fn type_id(&self) -> NFTFieldTypeId {
		match self {
			NFTField::I32(_) => 0,
			NFTField::U8(_) => 1,
			NFTField::U16(_) => 2,
			NFTField::U32(_) => 3,
			NFTField::U64(_) => 4,
			NFTField::U128(_) => 5,
			NFTField::Bytes32(_) => 6,
		}
	}
	/// Return a new NFTField with a default value for the given type id
	pub fn default_from_type_id(type_id: NFTFieldTypeId) -> Self {
		match type_id {
			0 => NFTField::I32(0),
			1 => NFTField::U8(0),
			2 => NFTField::U16(0),
			3 => NFTField::U32(0),
			4 => NFTField::U64(0),
			5 => NFTField::U128(0),
			6 => NFTField::Bytes32([0_u8; 32]),
			_ => panic!("impossible"),
		}
	}
}