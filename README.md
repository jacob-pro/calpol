# Calpol

[![Build status](https://github.com/jacob-pro/calpol/actions/workflows/rust.yml/badge.svg)](https://github.com/jacob-pro/calpol/actions)

A multi-Service health monitor.

This is a service that runs a suite of tests against remote servers at a regular interval. Tests are marked as failing
if consecutive runs reach the configured failure threshold. Failing tests will trigger SMS and/or email notifications
to be sent to users.

## Getting Started

The server can be launched with `calpol -c config/config.toml server`. 
([See the example config file](./config/example.toml)).

The server binary can also be used to create the very first user account: 
`calpol -c config/config.toml create-user --email $EMAIL --password $PASSWORD`.
(Subsequent users can be created and updated via the API)

Note: running `calpol` with a valid configuration will automatically run the database migrations.

### Deployment

The server is available as a docker image: `ghcr.io/jacob-pro/calpol:latest`.

Note the API is plain-HTTP - it is intended to be deployed behind a reverse proxy.

See the [example](./config/docker-compose-example.yml) for how to deploy with docker-compose.

### Using the CLI

A [CLI](./calpol-cli) is provided for easy use communication with the server's REST API.

Using `calpol-cli --help` should explain everything, but here are some example commands:

```bash
## Prompts for server URL, email, and password. Saves the profile to AppData (or equivalent)
calpol-cli sessions login

## Enable sms notifications on currently logged in account
calpol-cli users update self --sms-notifications true --phone-number +4400000000

## Create another user - they will be sent a password reset token
calpol-cli users create $NAME $EMAIL 

## A user can then consume a password reset token to set their password
calpol-cli password-reset submit --url $SERVER_URL --token $TOKEN

## Create a test (or update the existing test by name)
cat $JSON | calpol-cli tests upsert

## View latest results for all tests
calpol-cli test-results list
```

### Example Tests

Test an HTTP server:

```json
{
  "name": "contoso_portal", 
  "enabled": true,
  "failure_threshold": 3,
  "config": {
    "type": "http",
    "ip_version": "both",
    "url": "https://contoso.com/portal",
    "follow_redirects": true,
    "expected_redirect_destination": "https://www.contoso.com/portal",
    "expected_code": 401,
    "verify_ssl": true,
    "method": "GET",
    "minimum_certificate_expiry_hours": 48
  }
}
```

Test an SMTP server:

```json
{
  "name": "contoso_smtp",
  "enabled": true,
  "failure_threshold": 2,
  "config": {
    "type": "smtp",
    "ip_version": "v4",
    "domain": "contoso.com",
    "encryption": "starttls",
    "smtp_server_type": "mail_transfer_agent",
    "minimum_certificate_expiry_hours": 24
  }
}
```

Test a TCP connection:

```json
{
  "name": "contoso_ssh",
  "enabled": true,
  "config": {
    "ip_version": "both",
    "type": "tcp",
    "host": "ssh.contoso.com",
    "port": 22
  }
}
```

## Limitations

This is an application designed purely for my personal use-case, and thus there are a number of limitations which
may make this less useful for other users:

- The User Management system whilst secure, is only designed for a very small organisation; all user accounts have 
  full privileges to create/update/delete other users on the server.
- There is only a CLI provided, which may not be the most friendly to use.
- There is a lot of missing documentation.
- Currently, limited to basic HTTP, SMTP and TCP tests.
- All tests are run on the same interval.
- There is no support for scaling this beyond a single server.

However, any pull requests to improve these would be welcome.

## Building on Windows

- Set `PQ_LIB_DIR` environment variable to `C:\Program Files\PostgreSQL\${VERSION}\lib`
- Add `C:\Program Files\PostgreSQL\${VERSION}\bin` to `PATH`
