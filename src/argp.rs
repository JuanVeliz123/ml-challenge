extern crate clap;
use crate::AppConfig;
use clap::{Arg, App};
use std::str::FromStr;

pub fn arg_parse() -> AppConfig {
    let matches = App::new("ml-challenge")
        .version("1.0.0")
        .author("Juan A. Veliz <velizjuanagustin@gmail.com>")
        .about("A program to fetch and persist mails from a gmail account")
        .arg(Arg::with_name("username")
            .short("u")
            .long("user")
            .value_name("USERNAME")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("password")
            .short("p")
            .long("pass")
            .value_name("PASSWORD")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("dbname")
            .short("n")
            .long("dbname")
            .value_name("DATABASE NAME")
            .takes_value(true))
        .get_matches();
    
    let username = matches.value_of("username").unwrap();
    let password = matches.value_of("password").unwrap();
    let dbname = matches.value_of("dbname").unwrap_or("emails.db");
    AppConfig {
        username: format!("recent:{}", String::from_str(username).unwrap()),
        password: String::from_str(password).unwrap(),
        db_name: String::from_str(dbname).unwrap(),
    }
}