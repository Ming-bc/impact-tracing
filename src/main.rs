extern crate redis;
extern crate mongodb;
extern crate base64;
extern crate serde;
extern crate futures;

mod tool;
mod db;
mod message;
// mod trace;

use db::pack_storage;

fn main() {
    println!("Efficient Traceback for EEMS via Message Packing!");
}