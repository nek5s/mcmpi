# MCMPI

A simple utility for installing and starting minecraft servers locally. 

## Usage

```
mcmpi <url> [--redownload] [--reinstall] [--keep-zip] [--start] [--out=<path>]
```

Options:

`--redownload`: Force downloading the file again.

`--reinstall`:  Force reinstalling the modpack.

`--keep-zip`:   Keep the downloaded zip file.

`--start`:      Start the server after installation.

`--eula`:       Agree to the eula.

`--out=<path>`: Specify the output folder name.

## Notes

- A folder with the same name as the modpack will be created
- `--start` needs `screen` to be installed
- `--start` will only start the server if `start.sh` or `launch.sh` is found

## License

All rights reserved.
