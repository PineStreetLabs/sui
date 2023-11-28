// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
mod narwhal {
    #![allow(clippy::derive_partial_eq_without_eq)]

    include!(concat!(env!("OUT_DIR"), "/narwhal.PrimaryToPrimary.rs"));
    include!(concat!(env!("OUT_DIR"), "/narwhal.PrimaryToWorker.rs"));
    include!(concat!(env!("OUT_DIR"), "/narwhal.WorkerToPrimary.rs"));
    include!(concat!(env!("OUT_DIR"), "/narwhal.WorkerToWorker.rs"));
}

use crate::Transaction;
use bytes::Bytes;

pub use narwhal::{
    primary_to_primary_client::PrimaryToPrimaryClient,
    primary_to_primary_server::{MockPrimaryToPrimary, PrimaryToPrimary, PrimaryToPrimaryServer},
    primary_to_worker_client::PrimaryToWorkerClient,
    primary_to_worker_server::{MockPrimaryToWorker, PrimaryToWorker, PrimaryToWorkerServer},
    worker_to_primary_client::WorkerToPrimaryClient,
    worker_to_primary_server::{MockWorkerToPrimary, WorkerToPrimary, WorkerToPrimaryServer},
    worker_to_worker_client::WorkerToWorkerClient,
    worker_to_worker_server::{MockWorkerToWorker, WorkerToWorker, WorkerToWorkerServer},
};
