use actix_web::{post, get, web, App, HttpServer, HttpResponse};
use dotenv::dotenv;
use std::env;
use sqlx::{PgPool, postgres::PgRow};
use chrono::{Utc, NaiveDateTime};
use serde::{Serialize, Deserialize};

#[derive(Debug, serde::Deserialize)]
struct BidRequest {
    product_id: i32,
    user_id: i32,
    amount: f64,
}

#[post("/place-bid")]
async fn place_bid(pool: web::Data<PgPool>, req: web::Json<BidRequest>) -> HttpResponse {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let current_price_result = sqlx::query("SELECT product_price FROM products WHERE id = $1 FOR UPDATE")
        .bind(req.product_id)
        .fetch_one(&mut tx)
        .await;

    let current_price: f64 = match current_price_result {
        Ok(row) => row.get("product_price"),
        Err(_) => {
            tx.rollback().await.ok();
            return HttpResponse::InternalServerError().finish();
        }
    };

    if req.amount <= current_price {
        tx.rollback().await.ok();
        return HttpResponse::BadRequest().body("Bid amount must be higher than the current price");
    }

    let update_result = sqlx::query("UPDATE products SET product_price = $1 WHERE id = $2")
        .bind(req.amount)
        .bind(req.product_id)
        .execute(&mut tx)
        .await;

    if let Err(_) = update_result {
        tx.rollback().await.ok();
        return HttpResponse::InternalServerError().finish();
    }

    let insert_result = sqlx::query("INSERT INTO bids (amount, user_id, product_id) VALUES ($1, $2, $3)")
        .bind(req.amount)
        .bind(req.user_id)
        .bind(req.product_id)
        .execute(&mut tx)
        .await;

    if let Err(_) = insert_result {
        tx.rollback().await.ok();
        return HttpResponse::InternalServerError().finish();
    }

    if let Err(_) = tx.commit().await {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}


#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Product {
    product_id:  String,
    owner: String,
    current_bid_owner: Option<String>,
    product_video: Option<String>,
    product_title: String,
    product_description: String,
    product_price: f64,
    sale_expiry: NaiveDateTime,
}


async fn get_products(pool: &PgPool) -> Result<Vec<Product>, sqlx::Error> {
    let products = sqlx::query_as::<_, Product>(
        r#"SELECT product_id, owner, current_bid_owner, product_video, product_title, product_description, product_price, sale_expiry
           FROM products
           WHERE sale_expiry > $1"#)
        .bind(Utc::now().naive_utc())
        .fetch_all(pool)
        .await?;
    
    Ok(products)
}

#[get("/products")]
async fn list_products(pool: web::Data<PgPool>) -> HttpResponse {
    match get_products(pool.get_ref()).await {
        Ok(products) => HttpResponse::Ok().json(products),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url).await.expect("Failed to create pool");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(list_products)
    })
    .bind("127.0.0.1:8080")?
    .keep_alive(75)
    .workers(num_cpus::get())
    .run()
    .await
}
