use coa_opencode::agent_service::{AgentInfo, AgentRunInput, AgentRunOutput, AgentService, ModelInfo, SkillInfo};
use coa_opencode::backend::OpencodeBackend;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

type SharedBackend = Arc<Mutex<dyn AgentService + Send + Sync>>;

#[tokio::main]
async fn main() {
    let addr: SocketAddr = std::env::var("SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()
        .expect("Invalid SERVER_ADDR");

    let backend: OpencodeBackend = OpencodeBackend::from_env();
    let shared = Arc::new(Mutex::new(backend));

    let list_agents = warp::path!("agents")
        .and(warp::get())
        .and(with_shared(shared.clone()))
        .and_then(|shared: SharedBackend| async move {
            let lock = shared.lock().await;
            match lock.list_agents().await {
                Ok(ids) => Ok(warp::reply::json(&ids)),
                Err(e) => Err(warp::reject::custom(Error::Internal(e.to_string()))),
            }
        });

    let get_agent = warp::path!("agents" / String)
        .and(warp::get())
        .and(with_shared(shared.clone()))
        .and_then(|id: String, shared: SharedBackend| async move {
            let lock = shared.lock().await;
            match lock.get_agent_info(&id).await {
                Ok(info) => Ok(warp::reply::json(&info)),
                Err(e) => Err(warp::reject::custom(Error::Internal(e.to_string()))),
            }
        });

    let update_agent = warp::path!("agents" / String)
        .and(warp::put())
        .and(warp::body::json())
        .and(with_shared(shared.clone()))
        .and_then(|id: String, patch: AgentInfo, shared: SharedBackend| async move {
            let mut lock = shared.lock().await;
            match lock.update_agent_config(&id, patch).await {
                Ok(()) => Ok(warp::reply::with_status("OK", warp::http::StatusCode::OK)),
                Err(e) => Err(warp::reject::custom(Error::Internal(e.to_string()))),
            }
        });

    let list_skills = warp::path!("skills")
        .and(warp::get())
        .and(with_shared(shared.clone()))
        .and_then(|shared: SharedBackend| async move {
            let lock = shared.lock().await;
            match lock.list_skills().await {
                Ok(skills) => Ok(warp::reply::json(&skills)),
                Err(e) => Err(warp::reject::custom(Error::Internal(e.to_string()))),
            }
        });

    let list_models = warp::path!("models")
        .and(warp::get())
        .and(with_shared(shared.clone()))
        .and_then(|shared: SharedBackend| async move {
            let lock = shared.lock().await;
            match lock.list_models().await {
                Ok(models) => Ok(warp::reply::json(&models)),
                Err(e) => Err(warp::reject::custom(Error::Internal(e.to_string()))),
            }
        });

    let run_agent = warp::path!("agents" / String / "run")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_shared(shared.clone()))
        .and_then(|id: String, input: AgentRunInput, shared: SharedBackend| async move {
            let lock = shared.lock().await;
            match lock.run_agent(&id, input).await {
                Ok(output) => Ok(warp::reply::json(&output)),
                Err(e) => Err(warp::reject::custom(Error::Internal(e.to_string()))),
            }
        });

    let execute_skill = warp::path!("skills" / String / "execute")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_shared(shared.clone()))
        .and_then(|id: String, input: serde_json::Value, shared: SharedBackend| async move {
            let lock = shared.lock().await;
            match lock.execute_skill(&id, input).await {
                Ok(result) => Ok(warp::reply::json(&result)),
                Err(e) => Err(warp::reject::custom(Error::Internal(e.to_string()))),
            }
        });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allow_headers(vec!["content-type"]);

    let routes = list_agents
        .or(get_agent)
        .or(update_agent)
        .or(list_skills)
        .or(list_models)
        .or(run_agent)
        .or(execute_skill)
        .with(cors)
        .with(warp::trace::request())
        .recover(handle_rejection);

    println!("Listening on http://{}", addr);
    warp::serve(routes).run(addr).await;
}

fn with_shared(shared: SharedBackend) -> impl Filter<Extract = (SharedBackend,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || shared.clone())
}

struct Error(String);

impl warp::reject::Reject for Error {}

async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    if let Some(e) = err.find::<Error>() {
        Ok(warp::reply::with_status(e.0.clone(), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    } else if err.is_not_found() {
        Ok(warp::reply::with_status("Not Found".to_string(), warp::http::StatusCode::NOT_FOUND))
    } else {
        Ok(warp::reply::with_status("Internal Server Error".to_string(), warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    }
}
