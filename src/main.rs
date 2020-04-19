mod site;
mod db;
use db::Database;
mod fsdb;

fn main() {
    println!("Hello, world!");
    let db = fsdb::FSDatabase::from_root("./fstest").unwrap();
    println!("{:?}", db.get_boards());
    println!("{:?}", db.get_board(1234));
    println!("{:?}", db.get_board(1244));
    println!("{:?}\n", db.get_original(1234, 5678));
    println!("{:?}", db.get_thread(1234, 5678));

}
