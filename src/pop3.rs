extern crate openssl;
extern crate regex;

use crate::tcpstream::TCPStreamType;
use crate::persistence::{persist_emails, EmailEntity};
use crate::AppConfig;

use std::net::TcpStream;
use std::str::FromStr;
use std::io::{BufReader, Error};
use openssl::ssl::{SslConnector, SslMethod};
use regex::Regex;

#[derive(Debug, PartialEq)]
enum POP3State {
    BEGIN,
    AUTHORIZATION,
    TRANSACTION,
    END,
}

pub struct POP3Client {
    config: AppConfig,
    ssl_stream: TCPStreamType,
    state: POP3State,
}

#[derive(Debug)]
struct POP3EmailBody {
    is_plain_text: bool,
    raw_body: String,
}

const DOT: u8 = 0x2E;
const CR: u8 = 0x0D;
const LF: u8 = 0x0A;

impl POP3Client {
    pub fn new(config: AppConfig) -> Result<POP3Client, Error> {
        println!("Connecting to pop.gmail.com at port 995...");
        let tcp_stream = match TcpStream::connect("pop.gmail.com:995") {
            Ok(stream) => stream,
            Err(_) => panic!("Could not connect to the server!"),
        };
        let connector = match SslConnector::builder(SslMethod::tls()) {
            Ok(builder) => builder.build(),
            Err(err) => panic!("There was a problem building the SSL connector: {}", err),
        };
        let ssl_stream = TCPStreamType::SSL(BufReader::new(
            match connector.connect("pop.gmail.com", tcp_stream) {
                Ok(ssl_stream) => ssl_stream,
                Err(_) => panic!("Error establishing a SSL stream connection!"),
            },
        ));
        Ok(POP3Client {
            config,
            ssl_stream,
            state: POP3State::BEGIN,
        })
    }
    
    pub fn fetch_emails(&mut self) {
        let mut buff = Vec::new();
        self.ssl_stream.read_until(LF, &mut buff).unwrap();
        println!("Got response from server");
        println!("Logging in with USER/PASS...");
        let username = &self.config.username.clone();
        let password = &self.config.password.clone();
        match self.send_command("USER", Some(username)) {
            Ok(_) => (),
            Err(_) => panic!("Error sending USER cmd"),
        };
        match self.send_command("PASS", Some(password)) {
            Ok(_) => (),
            Err(_) => panic!("Error sending PASS cmd"),
        };
        println!("Logged in succesfully");
        self.state = POP3State::TRANSACTION;

        println!("Fetching emails...");
        let number_of_mails = match self.send_command("STAT", None) {
            Ok(stats) => {
                let test = &stats[0][4..5];
                println!("There are {} mails available", test);
                test.parse::<u8>().unwrap()
            },
            Err(_) => panic!("Could not parse STATS command"),
        };
        let mut email_entities_to_persist: Vec<EmailEntity> = Vec::new();
        for mail_number in 1..number_of_mails+1 {
            match self.send_command("RETR", Some(&mail_number.to_string())) {
                Ok(res) => {
                    let subject: String = res.clone().into_iter()
                        .filter(|line| line.starts_with("Subject:"))
                        .collect::<Vec<String>>()
                        .pop()
                        .unwrap();

                    let msg_from: String = res.clone().into_iter()
                        .filter(|line| line.starts_with("From:"))
                        .collect::<Vec<String>>()
                        .pop()
                        .unwrap();

                    let msg_date: String = res.clone().into_iter()
                        .filter(|line| line.starts_with("Date:"))
                        .collect::<Vec<String>>()
                        .pop()
                        .unwrap();

                    let email_body = match self.try_parsing_email_body(
                        &res.clone(),
                        mail_number.to_string()
                    ) {
                        Ok(pop3_body) => pop3_body.raw_body,
                        Err(_) => panic!("Could not parse email body!"),
                    };

                    println!("Analyzing mail from {}", &msg_from[5..]);
                    let should_persist = subject.contains("DevOps") || email_body.contains("DevOps");
                    if should_persist {
                        email_entities_to_persist.push(
                            EmailEntity {
                                date_received: String::from_str(&msg_date[5..]).unwrap(),
                                from: String::from_str(&msg_from[5..]).unwrap(),
                                subject: String::from_str(&subject[8..]).unwrap(),
                            }
                        )
                    }
                },
                Err(_) => panic!("Unable to parse response"),
            };
        }
        println!("Closing TCP connection...");
        self.quit().unwrap();

        println!("Persisting {} emails in database..", email_entities_to_persist.len());
        persist_emails(email_entities_to_persist, self.config.db_name.clone()).unwrap();
        println!("All done!")
    }

    pub fn send_command(&mut self, command: &str, param: Option<&str>) -> Result<Vec<String>, Error> {
        let is_multiline = match command {
            "LIST" | "UIDL" => param.is_none(),
            "RETR" | "TOP" => true,
            _ => false,
        };
        let command = match param {
            Some(x) => format!("{} {}", command, x),
            None => command.to_string(),
        };
        self.ssl_stream.write_string(&command).unwrap();
        Ok(self.read_response(is_multiline))
    }

    fn read_response(&mut self, is_multiline: bool) -> Vec<String> {
        //TODO static/const regex
        let response_regex: Regex = Regex::new(r"^(?P<status>\+OK|-ERR) (?P<statustext>.*)").unwrap();
        let mut response_data: Vec<String> = Vec::new();
        let mut buff = Vec::new();
        let mut complete;

        self.ssl_stream.read_until(LF, &mut buff).unwrap();
        response_data.push(parse_to_utf8_without_crlf(&buff).unwrap());
        let status_line = response_data[0].clone();
        let response_groups = response_regex.captures(&status_line).unwrap();
        match response_groups
            .name("status")
            .ok_or("Regex match failed").unwrap()
            .as_str()
        {
            "+OK" => complete = false,
            "-ERR" => panic!(response_groups["statustext"].to_string()),
            _ => panic!("Un-parseable response"),
        };

        while !complete && is_multiline {
            buff.clear();
            self.ssl_stream.read_until(LF, &mut buff).unwrap();
            if buff == [DOT,CR,LF] {
                complete = true;
            } else {
                let buff_string = match parse_to_utf8_without_crlf(&buff) {
                    Ok(res) => res,
                    _ => panic!("Could not parse buffer to UTF-8!")
                };
                response_data.push(buff_string);
            }
        }
        response_data
    }

    fn quit(&mut self) -> Result<(), Error> {
        assert!(self.state == POP3State::AUTHORIZATION || self.state == POP3State::TRANSACTION);
        let _ = self.send_command("QUIT", None);
        self.state = POP3State::END;
        self.ssl_stream.shutdown();
        Ok(())
    }

    fn try_parsing_email_body(&mut self, buffer: &Vec<String>, mail_number: String) -> Result<POP3EmailBody, Error> {
        let multipart_alternative_index = buffer.into_iter()
            .position(|line| line.starts_with("Content-Type: multipart/"));
        if multipart_alternative_index != None {
            let boundary_line: String = match buffer.clone()
                .into_iter()
                .filter(|line| line.contains("boundary=\""))
                .collect::<Vec<String>>()
                .pop() {
                    Some(x) => x,
                    None => panic!("Found multipart Content-Type but no boundary field!"),
                };
            let boundary_start_index = boundary_line.find("boundary=").unwrap() + 10;
            let boundary = &boundary_line[boundary_start_index..boundary_line.len()-1];
            let body_alternatives = POP3Client::get_body_alternatives(buffer, boundary);
            Ok(body_alternatives.into_iter()
                .filter(|alt| alt.is_plain_text)
                .collect::<Vec<POP3EmailBody>>()
                .pop()
                .unwrap())
        } else {
            //If we cannot find boundaries, that means we've received a plain text e-mail,
            //hence we'll have to send a TOP command to find out where the body begins
            //We should probably do this check with more lines, but for practical purposes
            //I'm just using 1
            let raw_body = match self.send_command("TOP", Some(&format!("{} 1", mail_number))) {
                Ok(mut res) => {
                    let last_line = res.pop().unwrap();
                    let body_begin_index = buffer.into_iter()
                        .position(|line| line == &last_line)
                        .unwrap();
                    buffer[body_begin_index..].concat()
                },
                Err(_) => panic!("Error trying to fetch TOP command!"),
            };
            Ok(POP3EmailBody{
                is_plain_text: true,
                raw_body,
            })
        }
    }
    
    fn get_body_alternatives(buffer: &Vec<String>, boundary: &str) -> Vec<POP3EmailBody> {
        let mut boundary_indexes: Vec<usize> = Vec::new();
        for (index, line) in buffer.iter().enumerate() {
            if line.contains(&format!("--{}", boundary)) {
                boundary_indexes.push(index);
            }
        }
        let mut body_alternatives: Vec<POP3EmailBody> = Vec::new();
        for (vector_index, b_index) in boundary_indexes.clone().iter().enumerate() {
            if vector_index < boundary_indexes.len()-1 {
                let is_plain_text = buffer[b_index+1].contains("text/plain");
                let b64_encoded = buffer[b_index+2].contains("base64");
                let mut raw_body = buffer[b_index+3..boundary_indexes[vector_index+1]].concat();
                if b64_encoded {
                    raw_body = String::from_utf8(base64::decode(raw_body).unwrap()).unwrap();
                }
                body_alternatives.push(POP3EmailBody {
                    is_plain_text,
                    raw_body,
                });
            }
        }
        body_alternatives
    }
}

fn parse_to_utf8_without_crlf(buffer: &Vec<u8>) -> Result<String, Error> {
    assert!(buffer.len() >= 2);
    match String::from_utf8((&buffer.clone()[0..buffer.len()-2]).to_vec()) {
        Ok(res) => Ok(res),
        _ => panic!("Error parsing utf8"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[test]
    fn parse_to_utf8_without_clrf__with_proper_vector__returns_correctly_parsed_string() {
        let utf8_vector = vec![0x68,0x6f,0x6c,0x61,CR,LF];
        let correct_result_as_string = String::from_str("hola").unwrap();
        assert_eq!(parse_to_utf8_without_crlf(&utf8_vector).unwrap(), correct_result_as_string)
    }

    #[test]
    #[should_panic(expected = "Error parsing utf8")]
    fn parse_to_utf8_without_clrf__with_bad_utf8_vector__should_panic() {
        let utf8_vector = vec![0xff,0xff,CR,LF];
        parse_to_utf8_without_crlf(&utf8_vector).unwrap();
    }

    #[test]
    #[should_panic(expected = "assertion failed: buffer.len() >= 2")]
    fn parse_to_utf8_without_clrf__with_empty_vector__should_fail_assertion() {
        let utf8_vector = vec![];
        parse_to_utf8_without_crlf(&utf8_vector).unwrap();
    }
}