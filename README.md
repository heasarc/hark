# HEASARC offline database browser
`hark` is a command line tool that allows you to search part of the HEASARC archival database and download the data.

Tables from active missions are included in the package. It therefore does not need to connect to the HEASARC servers.
The tool also provide functions that allow you to download data directly from the cloud on Amazon Web Services (AWS).

# Installation
`hark` is a single binary file that can be installed with:

```sh
curl -sSL https://raw.githubusercontent.com/HEASARC/hark/gh/install.sh | sh
```

This will install the `hark` binary file in the folder in which the curl command was run. If you want to install it
in another location instead, run:
```sh
curl -sSL https://raw.githubusercontent.com/HEASARC/hark/gh/install.sh | sh -s -- --install-dir /my/custom/location/bin
```

Make sure the install location is in you `$PATH` variable. Then run: `hark`.

# Usage
Once installed, type `?` to print the help message:
```
hark> ?

hark: HEASARC archive offline explorer.

Commands
     list-tables: List supported tables
    list-columns: List columns of a table
     query-table: Query a specific table
    aws-download: Download data from AWS
      -----------
            help: Show help message (also: h, ?).
                  Use help command-name for command help
     self-update: Update hark to the latest version!
           about: About hark!
            exit: Exit (also: quit, q)
```

`hark` currently has four main commands:

- `list-tables` to list the tables supported by the applications.
- `list-columns ...` to list the names and descriptions of the columns in a given table.
- `query-table ...`: Query a specific table using a circular search region centered at some RA,DEC position,
    with some radius. A target name can also be supplied; name resolution requires an internet connection.
- `aws-download ...`: Download the data from the AWS. The passed URI is what you get when calling `query-tables`
    and requesting the data products.
- `self-update`: Update `hark` to the latest version to pick up the latest features and the most recent table updates.

# Examples
## list-columns
  - List the default columns for the NICER master catalog:
     `list-columns nicermastr`

  - List all columns for the SWIFT master catalog:
    `list-columns swiftmastr all`
## query-table
  - Query nicermastr around position 182.6,39.4 using the default radius:
    `query-table nicermastr 182.6,39.4`

  - Query numaster around position 182.6,39.4 and radius 40 arcmin:
    `query-table numaster 182.6,39.4 40`

  - Query numaster around position 182.6,39.4 for specific columns:
    `query-table numaster 182.6,39.4 ra,dec,name`

  - Query numaster around position 182.6,39.4 for specific columns and add product links:
    `query-table numaster 182.6,39.4 ra,dec,name products`

  - Query xmmmaster around position 182.6,39.4 for default columns and add product links:
     `query-table xmmmaster 182.6,39.4 products`

  - Query xmmmaster around the resolved coordinates of M31:
    `query-table xmmmaster M31`

  - Query a target name containing spaces:
    `query-table xmmmaster "NGC 1300" 40`
    
## aws-download
  - Download SWIFT obsid 000037258040:
     `aws-download s3://nasa-heasarc/swift/data/obs/2015_12/00037258040`

# FAQ
- Does `query-table` support source names?
Yes. The CDS name resolver looks up the target before the catalog search. This requires an internet connection;
coordinate-based searches remain available offline. Quote names containing spaces, for example
`query-table xmmmaster "NGC 1300"`.

- What units should I use?
RA and DEC are in degrees. The radius is in arc minutes.

- Is there any way to do a query-table from the command line and save the output?
```sh
hark <<EOF > output.txt
query-table nicermastr 182.6,39.4
EOF
```

- Why do I not see the most recent observations when I search in `hark`?
The observation tables are updated continuously as observations are taken.
Run the `self-update` command inside `hark` to check for and install the most
recent features and table updates.