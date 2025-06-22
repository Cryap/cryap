use db::common::nodeinfo::NodeInfoUsage;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInfoWellKnown {
    pub links: Vec<NodeInfoWellKnownLinks>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInfoWellKnownLinks {
    pub rel: Url,
    pub href: Url,
}

// Nodeinfo spec: http://nodeinfo.diaspora.software/docson/index.html#/ns/schema/2.1
#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfo {
    pub version: Option<String>,
    pub software: Option<NodeInfoSoftware>,
    pub protocols: Option<Vec<String>>,
    pub usage: Option<NodeInfoUsage>,
    pub open_registrations: Option<bool>,
    pub services: Option<NodeInfoServices>,
    pub metadata: Option<NodeInfoMetadata>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum NodeInfoSoftware {
    Version2_0(NodeInfoSoftware2_0),
    Version2_1(NodeInfoSoftware2_1),
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct NodeInfoSoftware2_0 {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct NodeInfoSoftware2_1 {
    pub name: Option<String>,
    pub version: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoServices {
    pub inbound: Option<Vec<String>>,
    pub outbound: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoMetadata {
    pub node_name: Option<String>,
    pub node_description: Option<String>,
}
