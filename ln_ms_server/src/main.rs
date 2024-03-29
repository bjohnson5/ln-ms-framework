// External Modules
use actix_files as fs;
use actix_web::{App, HttpServer};
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url};

/*
 * This is the main entry point for the simulation web server. This module exposes the ln_ms_lib library api to a web client.
 */

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            api::get_sim,
            api::get_network,
            api::create_sim,
            api::create_node,
            api::create_channel,
            api::create_event,
            api::run_sim,
            api::import_network,
            api::export_network,
            api::import_transactions
        ),
        components(schemas(
            api::CreateSimRequest,
            api::CreateNodeRequest,
            api::CreateChannelRequest,
            api::CreateEventRequest,
            api::RunSimulationRequest,
            api::ImportNetworkRequest,
            api::ExportNetworkRequest,
            api::ImportTransactionsRequest
        )
        )
    )]
    struct ApiDoc;

    println!("View simulation swagger api here: http://localhost:8080/swagger-ui/index.html");
    println!("View simulated network here: http://localhost:8080/network_monitor");
    println!("View simulation results here: http://localhost:8080/results");

    HttpServer::new(move || {
        App::new()
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .service(api::network_monitor)
            .service(api::get_sim)
            .service(api::get_network)
            .service(api::create_sim)
            .service(api::create_node)
            .service(api::create_channel)
            .service(api::create_event)
            .service(api::run_sim)
            .service(api::import_network)
            .service(api::results)
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![
                (
                    Url::new("api", "/api-doc/openapi.json"), 
                    ApiDoc::openapi()
                )
                ])
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

pub mod api {
    // Project Modules
    use ln_ms_lib::LnSimulation;
    use ln_ms_lib::sim_results::SimResults;

    // Standard Modules
    use std::thread;

    // External Modules
    use actix_web::{get, post, HttpResponse, Responder, Result, web::{Json, Path}};
    use serde::{Deserialize, Serialize};
    use utoipa::{ToSchema};

    // Gets the network graph in order to show it in the browser
    #[get("/network_monitor")]
    pub async fn network_monitor() -> Result<HttpResponse> {
        Ok(HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(include_str!("../static/network_monitor.html")))
    }

    // Get the results of a simulation
    #[get("/results")]
    pub async fn results() -> Result<HttpResponse> {
        unsafe {
            match &RESULTS {
                Some(res) => {
                    let html = res.get_results_page();
                    Ok(HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(html))
                },
                None => Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body("could not get results"))
            }
        }
    }
    /* 
     * TODO: this will not be a global variable, each endpoint will get the LnSimulation object from the database
     * - for simplicity it will be used as a global variable right now in order to demonstate the use case
     */
    static mut SIM: Option<LnSimulation> = None;
    static mut RESULTS: Option<SimResults> = None;

    // TODO: implement
    #[utoipa::path(
        params(
            ("sim_name", description = "The name of the simulation to get information about")
        ),
        responses(
            (status = 200, description = "Successfully got information about a simulation", body = String),
        )
    )]
    #[get("/get_sim/{sim_name}")]
    pub async fn get_sim(sim_name: Path<String>) -> impl Responder {
        let sim_name = sim_name.into_inner();
        let response = String::from("Simulation Name: ") + &sim_name;
        HttpResponse::Ok().body(response)
    }

    // TODO: implement
    #[utoipa::path(
        params(
            ("sim_name", description = "The name of the simulation to get information about")
        ),
        responses(
            (status = 200, description = "Successfully got information about a simulation network", body = String),
        )
    )]
    #[get("/get_network/{sim_name}")]
    pub async fn get_network(sim_name: Path<String>) -> impl Responder {
        let _sim_name = sim_name.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_ref() {
                Some(s) => { 
                    let network_json = s.get_runtime_network_graph();
                    HttpResponse::Ok().content_type("text/json; charset=utf-8").body(network_json)
                }
                None => HttpResponse::Ok().body("")
            }
        }
    }

    // A request to create a new simulation
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct CreateSimRequest {
        name: String,
        duration: u64,
        num_nodes: u64
    }

    #[utoipa::path(
        request_body = CreateSimRequest,
        responses(
            (status = 200, description = "Successfully created a new simulation", body = String)
        )
    )]
    #[post("/create_sim")]
    pub async fn create_sim(req: Json<CreateSimRequest>) -> impl Responder {
        let create_sim_req = req.into_inner();
        unsafe {
            // TODO: create the simulation object and save it to a database and do not use a static global unsafe variable
            SIM = Option::Some(LnSimulation::new(create_sim_req.name, create_sim_req.duration, create_sim_req.num_nodes));
        }
        HttpResponse::Ok().body("Created Simulation")
    }

    // A request to create a new node and add it to the simulation
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct CreateNodeRequest {
        name: String,
        initial_balance: u64,
        running: bool
    }

    #[utoipa::path(
        request_body = CreateNodeRequest,
        responses(
            (status = 200, description = "Successfully created a new node", body = String),
            (status = 404, description = "Simulation not found", body = String)
        )
    )]
    #[post("/create_node")]
    pub async fn create_node(req: Json<CreateNodeRequest>) -> impl Responder {
        let create_node_req = req.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_mut() {
                Some(s) => { 
                    s.create_node(create_node_req.name, create_node_req.initial_balance, create_node_req.running);
                    HttpResponse::Ok().body("Created Node")
                }
                None => HttpResponse::NotFound().body("Simulation not found, try creating a new simulation before creating a node")
            }
        }
    }

    // A request to create a new channel and add it to the simulation
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct CreateChannelRequest {
        src_name: String,
        dest_name: String,
        amount: u64,
        id: u64
    }

    #[utoipa::path(
        request_body = CreateChannelRequest,
        responses(
            (status = 200, description = "Successfully created a new channel", body = String),
            (status = 404, description = "Simulation not found", body = String)
        )
    )]
    #[post("/create_channel")]
    pub async fn create_channel(req: Json<CreateChannelRequest>) -> impl Responder {
        let create_channel_req = req.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_mut() {
                Some(s) => {
                    s.create_channel(create_channel_req.src_name, create_channel_req.dest_name, create_channel_req.amount, create_channel_req.id);
                    HttpResponse::Ok().body("Created Channel")
                }
                None => HttpResponse::NotFound().body("Simulation not found, try creating a new simulation before creating a channel")
            }
        }
    }

    // A request to create a new event and add it to the simulation
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct CreateEventRequest {
        event_type: String,
        src_name: String,
        dest_name: String,
        amount: u64,
        time: u64,
        channel_id: u64
    }

    #[utoipa::path(
        request_body = CreateEventRequest,
        responses(
            (status = 200, description = "Successfully created a new event", body = String),
            (status = 404, description = "Simulation not found", body = String)
        )
    )]
    #[post("/create_event")]
    pub async fn create_event(req: Json<CreateEventRequest>) -> impl Responder {
        let create_event_req = req.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_mut() {
                Some(s) => {
                    // TODO: this will need to be much more generic and the request will need to only allow supported events
                    if create_event_req.event_type == "NodeOfflineEvent" {
                        s.create_stop_node_event(create_event_req.src_name, create_event_req.time);
                    } else if create_event_req.event_type == "NodeOnlineEvent"{
                        s.create_start_node_event(create_event_req.src_name, create_event_req.time);
                    } else if create_event_req.event_type == "OpenChannelEvent"{
                        s.create_open_channel_event(create_event_req.src_name, create_event_req.dest_name, create_event_req.amount, create_event_req.time, create_event_req.channel_id);
                    } else if create_event_req.event_type == "CloseChannelEvent"{
                        s.create_close_channel_event(create_event_req.src_name, create_event_req.channel_id, create_event_req.time);
                    } else if create_event_req.event_type == "TransactionEvent" {
                        s.create_transaction_event(create_event_req.src_name, create_event_req.dest_name, create_event_req.amount, create_event_req.time);
                    }
                    HttpResponse::Ok().body("Event Created")
                }
                None => HttpResponse::NotFound().body("Simulation not found, try creating a new simulation before creating an event")
            }
        }
    }

    // A request to start the simulation
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct RunSimulationRequest {
        name: String,
        nigiri: bool
    }

    #[utoipa::path(
        request_body = RunSimulationRequest,
        responses(
            (status = 200, description = "Successfully started a simulation", body = String),
            (status = 404, description = "Simulation not found", body = String)
        )
    )]
    #[post("/run_sim")]
    pub async fn run_sim(req: Json<RunSimulationRequest>) -> impl Responder {
        let run_sim_request = req.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_mut() {
                Some(s) => {
                    thread::spawn(move || {
                        let res = s.run(run_sim_request.nigiri);
                        match res {
                            Ok(r) => {
                                RESULTS = Some(r);
                            },
                            Err(_) => {}
                        }
                    });
                    HttpResponse::Ok().body(String::from("Running Simulation: ") + &run_sim_request.name)
                }
                None => HttpResponse::NotFound().body("Simulation not found, try creating a new simulation before running a simulation")
            }
        }
    }

    // A request to import a network definition from a file
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct ImportNetworkRequest {
        filename: String,
        import_map: String
    }

    // TODO: implement
    #[utoipa::path(
        request_body = ImportNetworkRequest,
        responses(
            (status = 200, description = "Network successfully imported", body = String),
            (status = 404, description = "Simulation not found", body = String)
        )
    )]
    #[post("/import_network")]
    pub async fn import_network(req: Json<ImportNetworkRequest>) -> impl Responder {
        let import_request = req.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_mut() {
                Some(s) => {
                    s.import_network(import_request.filename, import_request.import_map);
                    HttpResponse::Ok().body("Network Imported")
                }
                None => HttpResponse::NotFound().body("Simulation not found, try creating a new simulation before importing a network definition")
            }   
        }
    }

    // A request to export a network definition from a file
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct ExportNetworkRequest {
        filename: String
    }

    // TODO: implement
    #[utoipa::path(
        request_body = ExportNetworkRequest,
        responses(
            (status = 200, description = "Network successfully exported", body = String),
            (status = 404, description = "Simulation not found", body = String)
        )
    )]
    #[post("/export_network")]
    pub async fn export_network(req: Json<ExportNetworkRequest>) -> impl Responder {
        let export_request = req.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_mut() {
                Some(s) => {
                    s.export_network(export_request.filename);
                    HttpResponse::Ok().body("Network Imported")
                }
                None => HttpResponse::NotFound().body("Simulation not found, try creating a new simulation before importing a network definition")
            }   
        }
    }

    // A request to import a list of transactions from a file
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct ImportTransactionsRequest {
        filename: String
    }

    // TODO: implement
    #[utoipa::path(
        request_body = ImportTransactionsRequest,
        responses(
            (status = 200, description = "Transactions successfully imported", body = String),
            (status = 404, description = "Simulation not found", body = String)
        )
    )]
    #[post("/import_transactions")]
    pub async fn import_transactions(req: Json<ImportTransactionsRequest>) -> impl Responder {
        let import_request = req.into_inner();
        unsafe {
            // TODO: get the simulation object from a database and do not use a static global unsafe variable
            match SIM.as_mut() {
                Some(s) => {
                    s.import_transactions(import_request.filename);
                    HttpResponse::Ok().body("Transactions Imported")
                }
                None => HttpResponse::NotFound().body("Simulation not found, try creating a new simulation before importing transactions")
            }   
        }
    }    
}
