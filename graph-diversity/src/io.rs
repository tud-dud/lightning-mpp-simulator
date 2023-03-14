use clap::error::Result;
use log::info;
use serde::Serialize;
use std::{
    error::Error,
    fs::{self, File},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Diversity {
    pub(crate) lambda: f32,
    pub(crate) diversity: f32,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    pub(crate) amount: usize,
    pub(crate) routing_metric: simlib::RoutingMetric,
    pub(crate) diversity: Vec<Diversity>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output(pub(crate) Vec<Results>);

impl Output {
    fn to_json_file(
        &self,
        routing_metric: String,
        output_path: PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        let mut file_output_path = output_path;
        file_output_path.push(format!("diversity-{}{}", routing_metric, ".json"));
        let file = File::create(file_output_path.clone()).expect("Error creating file.");
        serde_json::to_writer_pretty(file, self).expect("Error writing to JSON file.");
        info!(
            "Diversity output written to {}.",
            file_output_path.display()
        );
        Ok(())
    }

    pub(crate) fn write(
        &self,
        routing_metric: String,
        output_path: PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(output_path.clone())
            .map(|()| -> Result<(), Box<dyn Error>> {
                info!("Writing JSON output files to {:#?}/.", output_path.clone());
                self.to_json_file(routing_metric, output_path.clone())?;
                Ok(())
            })
            .ok();
        Ok(())
    }
}
