# address-propensity

This project demonstrates the combination of loading (with some cleansing) of property and loan 
propensity scoring data into a database and a simple score query by zipcode endpoint. Two 
executables (loader and server) are built.

See below for steps to execute a full demo. *The demo is defined to run completely within docker 
containers so that it can be run without the need to install the Rust toolchain.*

## loader
The <code>loader</code> executable takes two subcommands (<code>property</code> and 
<code>propensity</code>) corresponding to the type of data to be loaded. Each has their own CSV 
structure expectations. Each command processed the targeted data file line by line. It checks 
whether the record exists in the database, and if it does not, saves the record after further 
validation. This line-by-line processing enables the <code>loader</code> to be re-run against 
the data file while only saving records not existing in the database; e.g., to reprocess after
fixing data format issues reported in a previous run. This approach was favored in this initial 
release for simplicity reasons and to optimize the user experience assuming manual corrections are 
anticipated.

See <code>./loader help</code> for usage information.

For the 10,000 record files, each command takes about a minute on my laptop. There are a couple of
techniques that can substantially improve performance:
* **Asynchronous Processing**: The tools used to implement the reading, validation, transformation, and saving data to the database support parallel, asynchronous processing in without blocking - even reading CSV files and database operations. Combining batching and nonn-blocking, asynchronous processing can substantially improved data load times. 
* **Batching Records**: Currently each record is saved in its own transaction. For handling all 
records in a single transaction, the <code>loader</code> completes in a few seconds, representing
potentially a couple orders of magnitude performance improvement. A single transaction is not 
recommended, however, due to the unbounded size of data files. Industry benchmarks (see Red Book) 
suggest for tables of the size of Properties, even batches of 25 represent a substantial 
improvement.
* **Caching Previous Identifiers**: We could maintain (or load at start) a bloom filter cache of
record identifiers (e.g., APN is a key field). Lookups for potential skips would only need to be 
done with a cache hit (since bloom filters do not have false negatives). This has the potential
to dramatically reduce database queries without impacting the re-processing feature.

### property loader
<code>./loader property resources/data/core_property_data.csv</code>
Loads the core property data from the <code>core_property_data.csv</code> file. Per discussion with
stakeholder, the core property data is considered to be the primary source of record for APN and 
address information. Propensity data from other sources key into this data.

Note the following regarding the behavior of the property loader: 
* APN format is validated for numeric characters or a '-' (and beginning and ending with a
 number). Further, APNs are front padded with '0' to be normalized to 14 characters. This format
 appears to be valid for this data set; however, based on my survey of assessor parcel numbers, their
 formats vary widely based on the direction of state and/or local governments. Obviously my current
 validation/format would need to be changed in order to apply nationally (and internationally).
* Address fields are generally not constrained in this process, since address validation is a
substantial capability that is out of score for this exercise.
* Optional geo-coordinates are validated to be floating-point numbers within latitude:[-90, 90] and 
longitude:[-180, 180].
* Optional area and total sq footage fields are validated as floating-point numerics.
* Optional bedroom count is validated to be a positive integer.
* Optional bathroom count is validated to be a positive, floating-point numeric.
* Other fields, such as the land use type are not validated.

### propensity loader
<code>./loader propensity resources/data/propensity_scores.csv</code>
Loads the propensity data from the <code>propensity_scores.csv</code> file. Per discussion with the 
stakeholder, this file represents the primary source of record only for propensity scores. Further,
additional fields, such as related to the address, are secondary and may be inconsistent. 
Address discrepancies with core property data are not reconciled. While discrepancies in address 
fields with the core property data undermine confidence in the propensity data source, because it
is currently unclear how discrepancies translate to propensity score quality, it is not in scope 
to further assess address deviations from core property data. See below for one technique to assess
address deviations in the future. Instead, since APN is the key correlating
identifier, the propensity APN is validated and normalized in the same manner as that for the core 
property set; i.e., accept numerics with dashes, strip out the dashes and front-pad to 14 character
length. 

In addition to the validation and serialization issues identified, <code>loader</code> will
identify the number of propensity records whose APN does not exist in the core property set. 
Because data could be loaded in batches, these records are still loaded into the database with the
hope they will find a match in the future. Future work is to clean out propensity records that do
not link to a core property record.

For future consideration: The address fields between the core property and propensity data sets do 
not match, so direct comparison may be difficult. Instead of trying compare fields, it may be 
simpler to first normalize addresses at a higher level: the mailing address. If we pull together 
the address fields for each set to create the mailing address string, we can then use different
string comparison techniques to evaluate differences; e.g., Levenshtein distance (good relative to
considering typos) and Hamming distance may be good to measure interpretability between the two sets.

## server
The <code>server</code> starts a simple REST endpoint that is used to query sorted (descending) 
propensity scores by zipcode. The endpoint also has a basic health check function.

### Query propensity scores for a zipcode
The endpoint returns a sorted array of addresses and their propensity score, sorted from highest to 
lowest for the given zipcode. The result set size can be limited. The following two query parameters
are supported: 
* required <code>zip</code> or <code>zipcode</code> or <code>zip_code</code>: zipcode to query for scores
* optional <code>limit</code>: constrain the result set size

For example to query the top three propensity scores for the 98121 zipcode:

    <code>curl --request GET '127.0.0.1:8000/propensity?zip_code=98121&limit=3'</code>

returns

<pre><code>[
    {
        "apn": "00006633050420",
        "score": 259,
        "address": {
            "address_line": {
                "street_number": "76",
                "street_name": "CEDAR",
                "street_suffix": "ST",
                "street_direction": "None"
            },
            "secondary_address_line": {
                "designator": "UNIT",
                "number": "509"
            },
            "city": "SEATTLE",
            "state_or_region": "WA",
            "zip_or_postal_code": {
                "code": "98121"
            },
            "locale": {
                "iso_3166_alpha_3": "USA",
                "official_name": "UNITED STATES OF AMERICA"
            }
        }
    },
    {
        "apn": "00007656901080",
        "score": 221,
        "address": {
            "address_line": {
                "street_number": "2600",
                "street_name": "2",
                "street_suffix": "AVE",
                "street_direction": "None"
            },
            "secondary_address_line": {
                "designator": "APT",
                "number": "612"
            },
            "city": "SEATTLE",
            "state_or_region": "WA",
            "zip_or_postal_code": {
                "code": "98121"
            },
            "locale": {
                "iso_3166_alpha_3": "USA",
                "official_name": "UNITED STATES OF AMERICA"
            }
        }
    },
    {
        "apn": "00003589004180",
        "score": 166,
        "address": {
            "address_line": {
                "street_number": "583",
                "street_name": "BATTERY",
                "street_suffix": "ST",
                "street_direction": "None"
            },
            "secondary_address_line": {
                "designator": "APT",
                "number": "510N"
            },
            "city": "SEATTLE",
            "state_or_region": "WA",
            "zip_or_postal_code": {
                "code": "98121"
            },
            "locale": {
                "iso_3166_alpha_3": "USA",
                "official_name": "UNITED STATES OF AMERICA"
            }
        }
    }
]</code></pre>

### health check
The endpoint also has a simple health check service that can be used to verify the server is up and 
accepting requests. The health check does not perform a full-system check; i.e., it does not verify 
that the database is operational, although the server will not start up if it cannot connect to the 
database. 

To query the health check:

    <code>curl --location --request GET 'localhost:8000/health_check'</code>

## configuration
Configuration for both the <code>server</code> and <code>loader</code> is loaded from a combination
of potential sources. (This mechanism was copied from some of my other personal projects, and will 
be consolidated into a separate crate.) Many possible file formats are supported, including 
<code>json</code>, <code>toml</code>, <code>yaml</code>, <code>hjson</code>, <code>ron</code>. 
<code>Yaml</code> files are used currently in this example.

The order of precedence for configuration sources is:
1. Base configuration either explicitly specified by the <code>-c|--config</code> option or
   a <code>application</code> configuration file found in the <code>resources</code> directory
   under the current working directory.
2. Environment specific overrides (for <code>local</code> or <code>production</code>) identified
   via the <code>APP_ENVIRONMENT</code> environment variable. This can be used to easily support
   different properties required for development and production; e.g., for database and application server
   <code>host</code> and <code>port</code> properties.
3. An optional secrets file is supported so you can avoid storing passwords and other secret
   information in your code repository. In practice, a CI pipeline would source secrets from a
   secure repository (e.g., a highly-restricted git repository or something like Vault) and included
   in the <code>server</code>'s deployment. For the <code>loader</code>, the user could specify a local file.
5. Finally, environment variables can be used to override configuration settings. They must
   begin with <code>APP__</code> and path elements separated by a <code>__</code> delimiter.

# Demo (entirely in Docker)
In order to see this system in work even if you do not have the Rust toolchain instaled on your 
machine, I've structured a demo to run in a simple set of docker containers. (I assume you have 
docker installed on your machine.)

The following steps will build, deploy and execute the demo on your machine. You'll need to jump in 
and out of docker containers, and the "deployed" containers are not what would normally be deployed
in production; e.g., I don't set <code>ENTRYPOINT</code> and the container size is larger to 
include parts needed for the demo.

(I've tested these steps on my MacBook Pro.)

Lines beginning with <code>> </code> are from your local prompt.

Clone this repository to your local system. From that directory (<code>address-propensity</code>)
follow these steps:

1. Start the Postgres database container.
<pre><code>> ./scripts/create_db.sh</code></pre>

2. Build Docker container used to initialize and load propensity database. This container is used 
only to provide a docker environment from which to set up the database and load data for the demo. 
Rust can cross-compile to many platforms (MacOs, Windows, limux, and many, many more), but without 
knowing your specific environment, the demo runs the loader executable in this container. The system 
uses sqlx, which is database connection library that, in addition to executing queries in the 
runtime, provides a schema migrations capability, which was used. Another cool feature of sqlx is 
that queries can be compile-time checked against the schema! 
<pre><code>> docker build --tag propensity-db-init --file Database.Dockerfile .</code></pre>

3. Build server executable container. The resulting container is pretty good for a production
environment. It is less than 100MB in size in a somewhat minimal base and with no root user. We can 
get it smaller - perhaps to around 20MB by building to a base such as scratch or alpine. 
<pre><code>> docker build --tag address-propensity --file Dockerfile .</code></pre>

4. Create the docker network used to link application and database containers
<pre><code>> docker network create --driver bridge --attachable --scope local propensity-network</code></pre>

5. Connect Postgres container to propensity-network
<pre><code>> docker network connect propensity-network propensity_postgres</code></pre>

6. Run db initiation container connected to the propensity-network.
<pre><code>> docker run -it -d --network propensity-network propensity-db-init</code></pre>

7. Enter propensity-db-init container. It returns the corresponding container id to be used in the 
following step. (note: only the first few characters of the container id are needed; e.g., I
normally refer to the first 3-4 characters.)
<pre><code>> docker exec -it [propensity-db-init-container-id] /bin/bash</code></pre>

8. In the container shell, run the sqlx migration, then exit (note root@:/# represents the shell 
prompt):
<pre><code>root@:/# sqlx migrate run</code></pre>

You should see:
<pre><code>
Applied 20210921035132/migrate create properties table (5.3027ms)
Applied 20210921050147/migrate add propensity table (3.9499ms)
Applied 20210921225524/migrate add apn indexes (4.782ms)
Applied 20210921225951/migrate add zipcode to propensity (1.7929ms)
</code></pre>

9. From the container's shell, load property data:
<pre><code>root@:/app# ./loader -s resources/secrets.yaml property resources/data/core_property_data.csv > property_load.log</code></pre>
You should see progress fill the screen and a summary of the load:
<pre><code>Saved 10000 records from "resources/data/core_property_data.csv" (0 skipped) with 0 issues found</code></pre>

10. From the container's shell, load propensity data:
<pre><code>root@:/app# ./loader -s resources/secrets.yaml propensity resources/data/propensity_scores.csv > propensity_load.log</code></pre>
You should see progress fill the screen and a summary of the load:
<pre><code>
 Saved 8699 records from "resources/data/propensity_scores.csv" (1301 skipped) with 1301 issues found:
	1301 missing scores
	2625 not in core properties (but still loaded)
 Propensity score visualization was saved to propensity_score_distribution.png.
 Zipcode propensity population visualization was save to score_zipcode_distribution.png.
</code></pre>

11. As reported, in addition to the summary status of the load, visualizations of the loaded
    propensity data are also generated and saved to the files
    <code>propensity_score_distribution.png</code> and <code>score_zipcode_distribution.png</code>.

12. From another terminal, you can copy the generated visualizations from the docker container:
<pre><code>> docker cp [propensity-db-init-container-id]:/propensity_score_distribution.png .</code></pre>
<pre><code>> docker cp [propensity-db-init-container-id]:/score_zipcode_distribution.png .</code></pre>

13. then exit propensity-db-init container shell
<pre><code>root@:/# exit</code></pre>

14. Kill propensity-db-init container.
<pre><code>> docker kill [propensity-db-init-container-id]</code></pre>

15. Now that the database and loaded is initialized we can start the executable environment:
<pre><code>> docker run -it -d --network propensity-network -p 8000:8000 address-propensity</code></pre>

16. In a separate host shell (can be the one used to pull visualizations), use curl to query zipcode
propensity scores:
<pre><code>> curl --location --request GET '127.0.0.1:8000/propensity?zip_code=98121&limit=3'</code></pre>
You should see:
<pre><code>[{"apn":"00006633050420","score":259,"address":{"address_line":{"street_number":"76","street_name":"CEDAR","street_suffix":"ST","street_direction":"None"},"secondary_address_line":{"designator":"UNIT","number":"509"},"city":"SEATTLE","state_or_region":"WA","zip_or_postal_code":{"code":"98121"},"locale":{"iso_3166_alpha_3":"USA","official_name":"UNITED STATES OF AMERICA"}}},{"apn":"00007656901080","score":221,"address":{"address_line":{"street_number":"2600","street_name":"2","street_suffix":"AVE","street_direction":"None"},"secondary_address_line":{"designator":"APT","number":"612"},"city":"SEATTLE","state_or_region":"WA","zip_or_postal_code":{"code":"98121"},"locale":{"iso_3166_alpha_3":"USA","official_name":"UNITED STATES OF AMERICA"}}},{"apn":"00003589004180","score":166,"address":{"address_line":{"street_number":"583","street_name":"BATTERY","street_suffix":"ST","street_direction":"None"},"secondary_address_line":{"designator":"APT","number":"510N"},"city":"SEATTLE","state_or_region":"WA","zip_or_postal_code":{"code":"98121"},"locale":{"iso_3166_alpha_3":"USA","official_name":"UNITED STATES OF AMERICA"}}}]</code></pre>

17. Have fun experimenting!
