# trustrl

URL manipulator tool.

---

This is a more-or-less rewrite of [trurl](https://github.com/curl/trurl). I liked the idea of `trurl`, a tiny tool to 
manipulate URLs so I rewrote it in Rust. While I was initially going to make this a one-to-one match in terms of CLI 
API, I ended up making a few changes along the way. While the functionality of both tools is almost identical, their 
APIs differ a bit.

# Compiling

Download [Rust](https://www.rust-lang.org/learn/get-started) and run:

```shell
cargo build --release
```

The output binary is in `./target/release/trustrl`.

# Tool usage

This tool takes either a single URL or a path to a file that contains a list of URLs. For every URL it will:

1. Apply a set of transformations. This can be things like changing the scheme or host, adding a new query string 
   element, changing the port, etc.
2. Render the URL. This uses two flavors:
    1. A template string like `The port is {port}`, which does exactly what you'd expect.
    2. Converts the URL into a JSON object that contains every component in the URL.

For example, rendering using a template string:

```shell
$ ./trustrl http://example.com/foo?a=b -t 'the path is {path}, the port is {port}'
the path is /foo, the port is 80
```

And rendering the same URL as a JSON object:

```shell
./trustrl http://example.com/foo?a=b -j | jq
{
  "url": "http://example.com/foo?a=b",
  "scheme": "http",
  "host": "example.com",
  "port": 80,
  "path": "/foo",
  "query": "a=b",
  "params": [
    {
      "key": "a",
      "value": "b"
    }
  ]
}
```

## Transformations

Transformations allow changing _something_ in each URL. For example:

```
$ ./trustrl example.com/foo --scheme https --port 1337 --append-path bar
https://example.com:1337/foo/bar
```

See the help for the full list of transformations.

## Template keys

The keys supported in the template string are:

* url
* scheme
* host
* port
* user
* password
* path
* query
* fragment

# Help

```
/trustrl -h
Usage: trustrl [OPTIONS] <URL|--urls-path <URLS_PATH>>

Arguments:
  [URL]  The URL to be used

Options:
      --urls-path <URLS_PATH>
          A path to a list of URLs to process
  -t, --template <TEMPLATE>
          The template to be used to render the URL [default: {url}]
  -j, --to-json
          Output URLs in JSON format
  -s, --scheme <SCHEME>
          Set the URL's scheme
  -H, --host <HOST>
          Set the URL's host
  -P, --port <PORT>
          Set the URL's port
  -p, --path <PATH>
          Set the URL's path
  -u, --user <USER>
          Set the URL's user
  -S, --password <PASSWORD>
          Set the URL's password
  -f, --fragment <FRAGMENT>
          Set the URL's fragment
  -r, --redirect <REDIRECT>
          Redirect the URL to a new path
  -a, --append-path <APPEND_PATH>
          Append a new segment at the end of the path
  -q, --append-query-string <APPEND_QUERY_STRING>
          Append a new query string pair
  -c, --clear-query-string
          Clear the query string
      --allow-query-string <ALLOW_QUERY_STRING>
          Keep the query string keys that match this regex
      --deny-query-string <DENY_QUERY_STRING>
          Remove the query string keys that match this regex
      --sort-query-string
          Sort query string
  -h, --help
          Print help
```
