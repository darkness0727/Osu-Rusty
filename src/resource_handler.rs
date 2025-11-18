use std::{fs, io, path::Path};

use bytes::Bytes;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::{Error, utils::save_file};

static ROOT_PATH: &str = "./resources/";

pub fn save_resource(category: ResourceCategory, name: &str, data_bytes: Bytes) -> Result<String, Error> {
    let path = category_path(category) + name;
    save_file(data_bytes, &path)?;
    Ok(path)
}

pub fn resource_exists(category: ResourceCategory, name: &str) -> bool {
    let path = category_path(category) + name;
    Path::new(&path).exists()
}

pub fn remove_resource(category: ResourceCategory, name: &str) -> io::Result<()> {
    let path = category_path(category) + name;
    fs::remove_file(path)
}

pub fn get_resource_path(category: ResourceCategory, name: &str) -> Option<String> {
    let path = category_path(category) + name;
    if Path::new(&path).exists() {
        Some(path)
    }
    else {
        None
    }
}

pub fn create_all_dir() -> io::Result<()>{
    for category in ResourceCategory::iter() {
        let path = category_path(category);
        fs::create_dir_all(&path)?;
    }
    Ok(())
}

fn category_path(category: ResourceCategory) -> String {
    let path = ROOT_PATH.to_string();
    match category {
        ResourceCategory::MapData => path + "map_data/",
        ResourceCategory::Score => path + "scores/",
    }
}

#[derive(EnumIter)] 
pub enum ResourceCategory {
    Score,
    MapData,
}