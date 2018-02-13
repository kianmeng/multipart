// Example usage with Rocket (https://rocket.rs)
//
// Direct integration is not provided at this time as it appears the Rocket folks would prefer
// to handle multipart requests behind the scenes.
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate multipart;
extern crate rocket;

use multipart::mock::StdoutTee;
use multipart::server::Multipart;
use multipart::server::save::Entries;
use multipart::server::save::SaveResult::*;

use rocket::{Data, Request};
use rocket::http::{ContentType, Status};
use rocket::response::Stream;
use rocket::response::status::Custom;

use std::io::{self, Cursor, Write};

#[post("/upload", data = "<data>")]
fn multipart_upload(cont_type: &ContentType, data: Data) -> Result<Stream<Cursor<Vec<u8>>>, Custom<String>> {
    // this can be implemented as a request guard but it seems like just more boilerplate
    if !cont_type.is_form_data() {
        return Err(Custom(
            Status::BadRequest,
            "Content-Type not multipart/form-data".into()
        ));
    }

    let (_, boundary) = cont_type.params().find(|&(k, v)| k == "boundary").ok_or_else(
            || Custom(
                Status::BadRequest,
                "`Content-Type: multipart/form-data` boundary param not provided".into()
            )
        )?;

    match process_upload(boundary, data) {
        Ok(resp) => Ok(Stream::from(Cursor::new(resp))),
        Err(err) => Err(Custom(Status::InternalServerError, err.to_string()))
    }
}

fn process_upload(boundary: &str, data: Data) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();

    // saves all fields, any field longer than 10kB goes to a temporary directory
    // Entries could implement FromData though that would give zero control over
    // how the files are saved
    match Multipart::with_body(data.open(), boundary).save().temp() {
        Full(entries) => process_entries(entries, &mut out)?,
        Partial(partial, reason) => {
            writeln!(out, "Request partially processed: {:?}", reason)?;
            if let Some(field) = partial.partial {
                writeln!(out, "Stopped on field: {:?}", field.source.headers)?;
            }

            process_entries(partial.entries, &mut out)?
        },
        Error(e) => return Err(e),
    }

    Ok(out)
}

fn process_entries(entries: Entries, out: &mut Vec<u8>) -> io::Result<()> {
    {
        let stdout = io::stdout();
        let tee = StdoutTee::new(&mut out, &stdout);
        entries.write_debug(tee)?;
    }

    writeln!(out, "Entries processed")
}

fn main() {
    rocket::ignite().mount("/", routes![multipart_upload]).launch();
}
