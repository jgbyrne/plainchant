mod site;
mod db;
mod fsdb;

fn main() {
    println!("Hello, world!");
    let db = fsdb::FSDatabase::from_root("./fstest");
    println!("{:?}", db);
}
