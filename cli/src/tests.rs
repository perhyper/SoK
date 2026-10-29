use super::*;
use crate::commands::{ingest_last_manifest, run_download_sources};
use crate::sources::sorted_open_access_statuses;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

mod cli;
mod contracts;
#[path = "tests/profiles.rs"]
mod discovery;
mod graph_snapshot;
mod identity;
mod import;
mod lint;
mod machine;
mod pipeline;
mod relations;
mod render;
mod runs;
mod smoke;
mod sources;
mod structured_import;
mod support;
mod validation;
#[path = "tests/work.rs"]
mod work_protocol;

use smoke::*;
use support::*;
