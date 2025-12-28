# Blitzserver

Tool for managing/hosting Dominions 5 servers

## Getting Started

1. [Download](https://github.com/blitzserver/blitzserver/releases) and run it.
3. It should open the settings page in your browser automatically, if it didn't, [it should be here](http://localhost:8080/admin).
4. Zip up the whole Dominions6 folder from your steamapps directory and upload it (this can take a few minutes).
5. [Upload a map](http://localhost:8080/maps) (you need at least one map before you can create a game, or it'll crap itself).
6. [Create a game](http://localhost:8080)

## Settings

All the settings are controlled through the admin page with one exception, `DATABASE_URL` will set the location of the sqlite file blitzserver stores *everything* in. It defaults to `./db`, which should create the `db` file next to the binary.
