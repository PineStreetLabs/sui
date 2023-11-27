// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::big_int::BigInt;
use super::move_object::MoveObject;
use async_graphql::*;

use sui_types::coin::Coin as NativeSuiCoin;

#[derive(Clone, SimpleObject)]
#[graphql(complex)]
pub(crate) struct Coin {
    pub id: ID,
    #[graphql(skip)]
    pub move_obj: MoveObject,
    #[graphql(skip)]
    pub balance: Option<BigInt>,
}

#[ComplexObject]
impl Coin {
    async fn balance(&self) -> Option<BigInt> {
        if let Some(existing_balance) = &self.balance {
            return Some(existing_balance.clone());
        }

        self.move_obj
            .native_object
            .data
            .try_as_move()
            .and_then(|x| {
                if x.is_coin() {
                    Some(NativeSuiCoin::extract_balance_if_coin(
                        &self.move_obj.native_object,
                    ))
                } else {
                    None
                }
            })
            .and_then(|x| x.expect("Coin should have balance."))
            .map(BigInt::from)
    }

    async fn as_move_object(&self) -> Option<MoveObject> {
        Some(self.move_obj.clone())
    }
}
