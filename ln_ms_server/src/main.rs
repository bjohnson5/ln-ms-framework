use actix_web::{App, HttpServer};
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url};

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    #[derive(OpenApi)]
    #[openapi(
        paths(
            api::hello,
            api::create_sim,
            api::create_node,
            api::create_channel,
            api::create_event,
            api::run_sim,
            api::import_network
        ),
        components(
            schemas(api::LnRequest)
        )
    )]
    struct ApiDoc;

    HttpServer::new(move || {
        App::new()
            .service(api::hello)
            .service(api::create_sim)
            .service(api::create_node)
            .service(api::create_channel)
            .service(api::create_event)
            .service(api::run_sim)
            .service(api::import_network)
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
    use actix_web::{get, post, HttpResponse, Responder};
    use serde::{Deserialize, Serialize};
    use utoipa::{ToSchema, IntoParams};
    
    #[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
    pub struct LnRequest {
        #[schema(example = 1)]
        id: i32,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug, IntoParams)]
    pub struct LnParams {
        id: i32,
    }

    #[utoipa::path(
        params(
            LnParams
        ),
        responses(
            (status = 200, description = "OK", body = String),
        )
    )]
    #[get("/")]
    pub async fn hello() -> impl Responder {
        HttpResponse::Ok().body("Hello world!")
    }

    #[utoipa::path(
        request_body = LnRequest,
        responses(
            (status = 200, description = "Successful", body = String)
        )
    )]
    #[post("/create_sim")]
    pub async fn create_sim(req_body: String) -> impl Responder {
        HttpResponse::Ok().body(req_body)
    }
    
    #[utoipa::path()]
    #[post("/create_node")]
    pub async fn create_node(req_body: String) -> impl Responder {
        HttpResponse::Ok().body(req_body)
    }

    #[utoipa::path()]
    #[post("/create_channel")]
    pub async fn create_channel(req_body: String) -> impl Responder {
        HttpResponse::Ok().body(req_body)
    }

    #[utoipa::path()]
    #[post("/create_event")]
    pub async fn create_event(req_body: String) -> impl Responder {
        HttpResponse::Ok().body(req_body)
    }

    #[utoipa::path()]
    #[post("/run_sim")]
    pub async fn run_sim(req_body: String) -> impl Responder {
        HttpResponse::Ok().body(req_body)
    }

    #[utoipa::path()]
    #[post("/import_network")]
    pub async fn import_network(req_body: String) -> impl Responder {
        HttpResponse::Ok().body(req_body)
    }
}
