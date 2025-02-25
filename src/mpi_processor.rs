use mpi::topology::Rank;
use mpi::traits::*;
use mpi::{self, request::WaitGuard};
use serde::{Deserialize, Serialize};

use crate::graph::{Graph, Edge, Node};
