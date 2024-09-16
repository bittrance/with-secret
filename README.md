# with-secret - reduce visibility of secrets in environment variables

![tests](https://github.com/bittrance/with-secret/actions/workflows/pr.yaml/badge.svg?branch=main)

with-secret is a CLI command to keep sensitive environment variables in the local secrets service (Linux) or keyring (MacOS) and allows you to execute commands with them.

The sensitive environment variables will still be visible while the process is running, but using with-secret means they won't be set in the shell you are normally using and they will not end up in you shell history file. Thus, a malicious process must be present when with-secret is exposing them on the executed command.

with-secret supports MacOS and Linux. Completions are provided for popular shells.

## Usage

### Setting environment variables

You can set sensitive env vars in several different ways. You can read them interactively. The profile will be created automatically if it does not yet exist:

```shell
$ with-secret set --profile prod-admin-api JWT
```

with-secret will prompt you on the terminal, like so:

```
Enter value for JWT: ******************************
```

Since so many cloud services helpfully provide cut-and-pastable bash snippets for its access environemnt variables, with-secret also understands a very small subset of bash, allowing you to do things like:

```shell
$ cat | with-secret set --profile aws-eu-prod
export AWS_ACCESS_KEY_ID = "ASIA..."
export AWS_SECRET_ACCESS_KEY = "1Omv..."
export AWS_SESSION_TOKEN = "..."
^D
```

You could even use `xclip` (or `pbpaste` on MacOS):

```shell
$ xclip -o | with-secret set --profile aws-eu-prod
```

You can also input the value of a single environment variable from stdin, which is useful if the value is large:

```shell
$ with-secret set --profile  CLIENT_CERTIFICATE < ./my-certificate.pem
```

### Using environment variables

Once you have stored the variables, you can ask with-secret to execute commands with all environemnt variables it is executed with, plus those environment variables that were stored on the profile:

```shell
$ with-secret use --profile aws-eu-prod terraform plan
```

You can also combine multiple profiles, which are merged in the order they are given:

```shell
$ with-secret use --profile aws-eu-prod --profile vault-eu-prod terraform plan
```

The profile can also be passed using environment variable `WITH_SECRET_PROFILE` which can be useful in combination with tools such as [envdir](https://cr.yp.to/daemontools/envdir.html).

### Listing profiles

with-secret depends on the excellent [keyring-rs](https://github.com/hwchen/keyring-rs) library which abstracts away the underlying secrets store. There is not yet support for listing. However, you should be able to view profiles in your ordinary keyring UI.

### Deleting profiles

For secrets that are long-lived, you may want to delete the profile after you are done using it:

```shell
$ with-secret delete --profile aws-eu-prod
```

## Contributing

with-secret is released under MIT license. Pull requests and issues are welcome.