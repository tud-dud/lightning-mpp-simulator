use super::{Output, PaymentInfo, Report, Results};
use crate::{sim::SimResult, WeightPartsCombi};

use log::{error, info};
use std::{
    error::Error,
    fs::{self, File},
    path::PathBuf,
};

impl Output {
    /// Converts a vector of SimResult to Result in preparation for output
    pub fn to_results_type(
        sim_result: &[SimResult],
        weight_parts_combi: WeightPartsCombi,
        run: u64,
    ) -> Results {
        let reports: Vec<Report> = sim_result
            .iter()
            .map(Report::sim_result_to_report)
            .collect();
        Results {
            scenario: weight_parts_combi,
            run,
            reports,
        }
    }

    pub fn write(
        results: Vec<Results>,
        output_path: PathBuf,
        run: u64,
    ) -> Result<(), Box<dyn Error>> {
        if Self::create_dir(&output_path).is_ok() {
            info!("Writing CSV files to {:#?}/.", output_path);
            let output = Output(results);
            output.to_json_file(output_path, run)?;
        } else {
            error!("Directory creation failed.");
        }
        Ok(())
    }

    fn to_json_file(&self, output_path: PathBuf, run: u64) -> Result<(), Box<dyn Error>> {
        let run_as_string = format!("{}{:?}", "run", run);
        let mut file_output_path = output_path;
        file_output_path.push(format!("{}{}", run_as_string, ".json"));
        let file = File::create(file_output_path.clone()).expect("Error creating file.");
        serde_json::to_writer_pretty(file, self).expect("Error writing to JSON file.");
        info!(
            "Simulation output written to {}.",
            file_output_path.display()
        );
        Ok(())
    }

    fn create_dir(path: &PathBuf) -> Result<(), std::io::Error> {
        fs::create_dir_all(path)
    }
}

impl Report {
    fn sim_result_to_report(sim_result: &SimResult) -> Self {
        let mut payments: Vec<PaymentInfo> = sim_result
            .successful_payments
            .iter()
            .map(PaymentInfo::from_payment)
            .collect();
        payments.extend(
            sim_result
                .failed_payments
                .iter()
                .map(PaymentInfo::from_payment),
        );
        Self {
            amount: crate::to_sat(sim_result.amount),
            total_num: sim_result.total_num,
            num_succesful: sim_result.num_succesful,
            num_failed: sim_result.num_failed,
            payments,
            adversaries: sim_result.adversaries.to_owned(),
            path_distances: sim_result.path_distances.0.to_owned(),
            anonymity_sets: sim_result.anonymity_sets.to_owned(),
        }
    }
}
