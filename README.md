# SSHack
A CTF platform built with rust that you access over ssh.
(WIP)

## Demo
![demo gif](./demo.gif)

## Install

For now, the easiest way to install this is by using the provided install script:
```
  # Install SSHack
  git clone https://github.com/d-z0n/SSHack.git
  cd SSHack
  ./install.sh
```
If this fails, make sure that you have cargo and sqlite3-devel installed.
If that doesn't solve it, feel free to submit an issue and if I (or you)
find a solution I will add it here.

## Usage
Add your flags to a toml file,
see the provided `flags.toml` to see how to do this.

Load the files into the flag database by running:
```
  sshack flags load -p <your_flag_file.toml>
```

Then run the server:
```
  sshack run
```

And connect to it from another host using:
```
  ssh -p 1337 <the_server_ip>
```


## Config
At the moment there are only two configurable values,
see `config.toml` to see how to use them.

The theme can be the name of any theme file in the themes folder (without .yaml),
you can also add new themes in `~/.config/sshack/themes` and use them.

## In the plans
Some things that I am looking into adding right now are:
 * Groups/Teams
 * Password guard for private ctfs
 * Better flag description rendering (highlight code, commands, network addresses)
 * More configuration (ports, etc)
 * Tags for flags (categories)
 * Profile pages
 * Sort/search/filter for flags (filter draft done)
 * Better loading of flags (not removing solves from users)

## Development

### v0.x.x

#### v.0.3.0 [ ]
 [ ] Allow for Admin file upload

 [X] Admin UI

 [X] Add better text editing

 [ ] Limit rerendering to make text selection better

 [ ] Teams (optional)

 [X] Docker container

 [ ] Spin up challange container on demand

 [X] leaderboard flag to open the ctf in leaderboard mode.

#### v.0.2.0 [X]
 [X] File download for challanges over sftp
 [X] Password protected ctfs (optional)
 [X] Make animations optional
 [X] About page (optional)
 [X] Include SSHack version
 [X] Improve instructions
 [X] Fix pasting bug

#### v0.1.0 [X]
 [X] Basic tui 
 [X] Basic management cli
 [X] Basic config file
 [X] SSH server
 [X] Authentication using public key
 [X] Browsing and submitting flags
 [X] Leaderboard
 [X] Basic filters
 [X] Basic animated banner


