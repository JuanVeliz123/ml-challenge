# ml-challenge
Program created for MercadoLibre coding challenge. It fetches emails from a
Gmail account using POP3 protocol and persists those with the word DevOps in
the subject or body.
 
## Setup dev environment
- Install rust/cargo (https://www.rust-lang.org/es/learn/get-started)
- Install OpenSSL: 
    - For linux/macOS: 
    ```
      # macOS
      $ brew install openssl@1.1
      
      # Arch Linux
      $ sudo pacman -S pkg-config openssl
      
      # Debian and Ubuntu
      $ sudo apt-get install pkg-config libssl-dev
      
      # Fedora
      $ sudo dnf install pkg-config openssl-devel
  ```
  - For windows: Either install through vcpkg or download the pre-compiled
  binaries (https://wiki.openssl.org/index.php/Binaries)
 
## Building/Running
To test the program you will need to have a gmail account that has POP3 enabled
in the configurations and that also allows less secure apps (https://myaccount.google.com/lesssecureapps)

To build the project you can either run `cargo build` and use it as a
console application. Running `ml-challenge.exe --user <email> --pass <password>`
You can run it with `--help` to get a full description of the args.

You can also run it directly with cargo using `cargo run -- --user <email> --pass <password>`
(Yes, you need to put two standalone dashes in the middle)

You can configure the final SQLite db name with `--dbname <name>`

## Note for Windows users
Sometimes in windows, having OpenSSL installed is not enough for the Rust
openssl-sys crate to detect the install path. Also, the binaries don't usually
come with the certificates, so you can download the Mozilla CA in PEM format
 at https://curl.haxx.se/docs/caextract.html

If you're experiencing SSL problems, you can tell openssl-sys where to look
with the following environmental variables:

```
OPENSSL_DIR=<dir_path>
SSL_CERT_FILE=<file_path>
```