use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use warp::Filter;
use std::sync::{Arc, Mutex};

// Struct for incoming request data
#[derive(Deserialize)]
struct RequestData {
    operation: String,
    file_path: Option<String>,
    df: Option<String>,   // Serialized DataFrame
    condition: Option<String>,
    columns: Option<Vec<String>>,
    n: Option<u32>,
}

// Struct for outgoing response data
#[derive(Serialize)]
struct ResponseData {
    df: String,  // Serialized DataFrame as result
}

// Function to execute pandas code in Python
fn handle_pandas_operation(py: Python, data: &RequestData) -> PyResult<String> {
    let locals = PyDict::new(py);

    // Handle CSV loading operation
    if data.operation == "load_csv" {
        let file_path = data.file_path.as_ref().unwrap();
        locals.set_item("file_path", file_path)?;
        py.run("import pandas as pd; df = pd.read_csv(file_path)", None, Some(locals))?;
    } else {
        locals.set_item("df", data.df.as_ref().unwrap())?;

        match data.operation.as_str() {
            "filter" => {
                let condition = data.condition.as_ref().unwrap();
                let code = format!("df = df.query('{}')", condition);
                py.run(&code, None, Some(locals))?;
            }
            "select_columns" => {
                let columns = data.columns.as_ref().unwrap().join("', '");
                let code = format!("df = df[['{}']]", columns);
                py.run(&code, None, Some(locals))?;
            }
            "head" => {
                let n = data.n.unwrap();
                let code = format!("df = df.head({})", n);
                py.run(&code, None, Some(locals))?;
            }
            _ => {}
        }
    }

    let result_df = locals.get_item("df").unwrap();
    let serialized_result: String = result_df.extract()?;
    Ok(serialized_result)
}

// Warp filter to handle incoming HTTP requests
async fn handle_request(data: RequestData) -> Result<impl warp::Reply, warp::Rejection> {
    Python::with_gil(|py| {
        let result = handle_pandas_operation(py, &data).unwrap();
        let response = ResponseData { df: result };
        Ok(warp::reply::json(&response))
    })
}

#[tokio::main]
async fn main() {
    let api = warp::path("execute")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handle_request);

    warp::serve(api).run(([127, 0, 0, 1], 3030)).await;
}
