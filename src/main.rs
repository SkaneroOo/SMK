use axum::{
    extract::{
        Extension,
        Multipart
    },
    routing::get,
    response::{
        Html,
        IntoResponse
    },
    Router
};
use sqlx::{
    migrate::MigrateDatabase,
    Pool, 
    Row, 
    Sqlite, 
    SqlitePool
};
use std::{
    fs::{
        File,
        remove_file
    },
    io::{
        Read,
        Write
    }
};

const DB_URL: &str = "sqlite://database.db";

type Conn = Pool<Sqlite>;

async fn uploader() -> Html<String> {
    let mut html = String::new();
    match File::open("public/uploader.html").expect("Failed to open file!").read_to_string(&mut html) {
        Ok(_) => (),
        Err(error) => panic!("error: {}", error),
    };
    Html(html)
}

#[axum::debug_handler]
async fn upload(Extension(conn): Extension<Conn>, mut multipart: Multipart) -> axum::response::Response {
    let mut set = vec![];
    let mut success = true;
    while let Some(field) = multipart
        .next_field().await.expect("Failed to get next field!")
    {
        if field.name().unwrap() != "fileupload" {
            continue;
        }
        
        // Grab the name
        let file_name = field.file_name()
            .unwrap().to_owned();

        let ext = file_name.split(".").last().unwrap();

        let count: i32 = sqlx::query("SELECT COUNT(*) FROM files;")
            .fetch_one(&conn)
            .await
            .unwrap()
            .get(0);

        // Create a path for the soon-to-be file
        let file_path = format!("files/{}.{}", count, ext);
        
        // Unwrap the incoming bytes
        let data = match field.bytes().await {
            Ok(data) => data,
            Err(_) => {
                success = false;
                break;
            },
        };

        // Open a handle to the file
        let mut file_handle = match File::create(file_path) {
            Ok(file) => file,
            Err(_) => {
                success = false;
                break;
            },
        };

        // if image::load_from_memory(&data).unwrap().write_to(&mut file_handle, image::ImageFormat::WebP).is_err() {
        //     return axum::response::Redirect::to("/upload?success=false").into_response();
        // }

        // // Write the incoming data to the handle
        if file_handle.write_all(&data).is_err() {
            success = false;
            break;
        }

        sqlx::query("INSERT INTO files (file, name) VALUES (?);").bind(format!("{}.{}", count, ext)).bind("").execute(&conn).await.unwrap();
        set.push((count, format!("{}.{}", count, ext)));
    }
    if !success {
        for ext in set {
            remove_file(format!("files/{}", ext.1)).unwrap();
            sqlx::query("DELETE FROM files WHERE id = ?;").bind(ext.0).execute(&conn).await.unwrap();
        }
        return axum::response::Redirect::to("/upload?success=false").into_response();
    }
    
    sqlx::query("INSERT INTO sets (id1, id2, id3) VALUES (?, ?, ?);").bind(set[0].0).bind(set[1].0).bind(set[2].0).execute(&conn).await.unwrap();

    axum::response::Redirect::to("/upload?success=true").into_response()
}

async fn database_setup(conn: &Pool<Sqlite>) {
    sqlx::query("CREATE TABLE IF NOT EXISTS files (id INTEGER PRIMARY KEY AUTOINCREMENT, file TEXT NOT NULL, name TEXT NOT NULL)")
        .execute(conn).await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS sets (id INTEGER PRIMARY KEY AUTOINCREMENT, id1 INTEGER NOT NULL REFERENCES files(id), id2 INTEGER NOT NULL REFERENCES files(id), id3 INTEGER NOT NULL REFERENCES files(id))")
        .execute(conn).await.unwrap();
    // conn.execute("CREATE TABLE IF NOT EXISTS files (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)").unwrap();
}

#[tokio::main]
async fn main() {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }
    let conn = SqlitePool::connect(DB_URL).await.unwrap();

    database_setup(&conn).await;

    let app = Router::new()
        .route("/upload", get(uploader).post(upload))
        .layer(Extension(conn));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await.expect("Failed to start listener!");
    
    axum::serve(listener, app)
        .await.expect("Failed to serve 'app'!");
}