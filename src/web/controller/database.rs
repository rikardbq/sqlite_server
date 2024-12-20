use actix_web::{post, web, HttpRequest, HttpResponse, Responder};

use crate::{
    core::{
        constants::{
            errors::{self, ErrorReason},
            queries,
        },
        db::{execute_query, AppliedQuery, QueryArg},
        state::AppState,
        util::{get_database_connections, get_query_result_claims, get_user_entry_and_access},
    },
    web::{
        jwt::{decode_token, generate_claims, generate_token, RequestMigration, Sub},
        request::{RequestBody, ResponseResult},
    },
};
/*
save for later testing


#[post("/{database}")]
async fn echo(
    data: web::Data<AppState<SqlitePool>>,
    path: web::Path<String>,
    req_body: String,
) -> impl Responder {
    let database = path.into_inner();
    let database_connections_clone: Arc<Mutex<HashMap<String, SqlitePool>>> =
        Arc::clone(&data.database_connections);
    let mut database_connections = database_connections_clone
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    println!(
        "hello {}, {}",
        database,
        database_connections.keys().count()
    );
    if !database_connections.contains_key(&database) {
        println!(
            "database connection is not opened, trying to open database {}",
            database
        );
        if let Ok(pool) =
            SqlitePool::connect(format!("sqlite:{}.db?mode=json", database).as_str()).await
        {
            database_connections.insert(database.clone(), pool);
        } else {
            println!();
        }
    }
    let db = &database_connections[&database];
    let result = execute_query(
        AppliedQuery::new(
            "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY NOT NULL, name VARCHAR(250) NOT NULL);"
        ),
        &db
    ).await.unwrap();
    println!("test {:?}", result);

    let result2 = sqlx
        ::query(
            "CREATE TABLE IF NOT EXISTS users2 (id INTEGER PRIMARY KEY NOT NULL, name VARCHAR(250), namu VARCHAR(250) NOT NULL);"
        )
        .execute(db).await
        .unwrap();
    println!("test {:?}", result2);

    let result3 = execute_query(
        AppliedQuery::new(
            "CREATE TABLE IF NOT EXISTS users5 (id INTEGER PRIMARY KEY NOT NULL, name VARCHAR(250), namu VARCHAR(250), asdf INTEGER);"
        ),
        &db
    ).await.unwrap();

    println!("test {:?}", result3);

    execute_query(
        AppliedQuery::new("INSERT INTO users5 (name, namu, asdf) VALUES (?, ?, ?)").with_args(
            vec![
                QueryArg::String("hello"),
                QueryArg::String("World"),
                QueryArg::Int(32),
            ],
        ),
        &db,
    )
    .await
    .unwrap();

    // let result4 = fetch_query(AppliedQuery::new("SELECT * FROM users5;"), &db).await.unwrap();
    // for (idx, row) in result4.iter().enumerate() {
    //     println!(
    //         "[{}]: {:?} {:?} {:?}",
    //         idx,
    //         row.get::<String, &str>("name"),
    //         row.get::<String, &str>("namu"),
    //         row.get::<i32, &str>("asdf")
    //     );
    // }

    // get the usr data
    let usr_clone: Arc<Mutex<HashMap<String, Usr>>> = Arc::clone(&data.usr);
    let usr = usr_clone.lock().unwrap_or_else(PoisonError::into_inner);

    let user_entry_for_id =
        &usr["b1a74559bea16b1521205f95f07a25ea2f09f49eb4e265fa6057036d1dff7c22"];
    println!("testing here usr = {:?}", user_entry_for_id);
    HttpResponse::Ok().body(req_body)
}


*/

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(handle_database_post);
    cfg.service(handle_database_migration_post);
}

#[post("/{database}")]
async fn handle_database_post(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
    req_body: web::Json<RequestBody>,
) -> impl Responder {
    let header_u_ = match req.headers().get("u_") {
        Some(hdr) => hdr.to_str().unwrap(),
        _ => {
            return HttpResponse::BadRequest().json(ResponseResult::new().error(format!(
                "{} [ {} ]",
                errors::ERROR_MISSING_HEADER,
                "u_"
            )));
        }
    };

    let database_name = path.into_inner();
    let (user_entry, user_access) =
        match get_user_entry_and_access(&data, header_u_, &database_name) {
            Ok(ue) => ue,
            Err(err) => return HttpResponse::Unauthorized().json(ResponseResult::new().error(err)),
        };

    let payload_token = &req_body.payload;
    let decoded_token = match decode_token(&payload_token, &user_entry.up_hash) {
        Ok(dec) => dec,
        Err(err) => {
            return HttpResponse::NotAcceptable()
                .json(ResponseResult::new().error(&format!("ERROR={:?}", err.kind())))
        }
    };

    let db = match get_database_connections(&data, &database_name).await {
        Ok(conn) => conn,
        Err(err) => return HttpResponse::NotFound().json(ResponseResult::new().error(err)),
    };

    let query_result_claims =
        match get_query_result_claims(decoded_token.claims, user_access, &db).await {
            Ok(res) => res,
            Err(err) => {
                if let Some(reason) = err.reason {
                    if reason == ErrorReason::UserNotAllowed {
                        return HttpResponse::Forbidden()
                            .json(ResponseResult::new().error(err.message));
                    } else if reason == ErrorReason::InvalidSubject {
                        return HttpResponse::NotAcceptable()
                            .json(ResponseResult::new().error(err.message));
                    }
                }

                return HttpResponse::InternalServerError()
                    .json(ResponseResult::new().error(err.message));
            }
        };

    let token = match generate_token(query_result_claims, &user_entry.up_hash) {
        Ok(t) => t,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .json(ResponseResult::new().error(&format!("ERROR={:?}", err.kind())))
        }
    };

    HttpResponse::Ok().json(ResponseResult::new().payload(token))
}

#[post("/{database}/m")]
async fn handle_database_migration_post(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
    req_body: web::Json<RequestBody>,
) -> impl Responder {
    let header_u_ = match req.headers().get("u_") {
        Some(hdr) => hdr.to_str().unwrap(),
        _ => {
            return HttpResponse::BadRequest().json(ResponseResult::new().error(format!(
                "{} [ {} ]",
                errors::ERROR_MISSING_HEADER,
                "u_"
            )));
        }
    };

    let database_name = path.into_inner();
    let (user_entry, user_access) =
        match get_user_entry_and_access(&data, header_u_, &database_name) {
            Ok(ue) => ue,
            Err(err) => return HttpResponse::Unauthorized().json(ResponseResult::new().error(err)),
        };

    if user_access < 2 {
        return HttpResponse::Forbidden()
            .json(ResponseResult::new().error(errors::ERROR_USER_NOT_ALLOWED));
    }

    let payload_token = &req_body.payload;
    let decoded_token = match decode_token(&payload_token, &user_entry.up_hash) {
        Ok(dec) => dec,
        Err(err) => {
            return HttpResponse::NotAcceptable()
                .json(ResponseResult::new().error(&format!("ERROR={:?}", err.kind())))
        }
    };

    let db = match get_database_connections(&data, &database_name).await {
        Ok(conn) => conn,
        Err(err) => return HttpResponse::NotFound().json(ResponseResult::new().error(err)),
    };

    let claims = decoded_token.claims;
    if claims.sub != Sub::MIGRATE {
        return HttpResponse::NotAcceptable()
            .json(ResponseResult::new().error(errors::ERROR_INVALID_SUBJECT));
    }

    let migration: RequestMigration = serde_json::from_str(&claims.dat).unwrap();
    // let bundled_args: Vec<QueryArg> = migrations.iter().flat_map(|m| m.parts.clone()).collect();
    // let bundled_queries: String = .migrations
    //     .iter()
    //     .flat_map(|m| m.query.chars())
    //     .collect();

    // start transaction here because we want to fail everything if we can't insert migration state
    // or if we fail any of these steps in any way
    let mut transaction = db.begin().await.unwrap();

    // create migration table and panic if fail
    match execute_query(
        AppliedQuery::new(queries::CREATE_MIGRATIONS_TABLE),
        &mut *transaction,
    )
    .await
    {
        Ok(_) => {
            match execute_query(
                AppliedQuery::new(queries::INSERT_MIGRATION).with_args(vec![
                    QueryArg::String(&migration.name),
                    QueryArg::String(&migration.query),
                ]),
                &mut *transaction,
            )
            .await
            {
                Ok(_) => {}
                Err(err) => {
                    let _ = &mut transaction.rollback().await;
                    panic!("{err}");
                }
            }
        }
        Err(err) => {
            let _ = &mut transaction.rollback().await;
            panic!("{err}");
        }
    };

    // apply migration
    let res = match execute_query(AppliedQuery::new(&migration.query), &mut *transaction).await {
        Ok(_) => {
            let _ = &mut transaction.commit().await;
            generate_claims(true.to_string(), Sub::DATA)
        }
        Err(err) => {
            let _ = &mut transaction.rollback().await;
            println!("{err}");
            generate_claims(false.to_string(), Sub::DATA)
        }
    };
    let token = generate_token(res, &user_entry.up_hash).unwrap();

    HttpResponse::Ok().json(ResponseResult::new().payload(token))
}
