# address-propensity

`> docker build --tag propensity-db-init --file Database.Dockerfile .`

`> docker build --tag address-propensity --file Dockerfile .`

`> docker network create --driver bridge --attachable --scope local propensity-network`

`> docker network connect propensity-network [postgres-container-name]`

`> docker run -it -d --network propensity-network propensity-db-init`

`> docker exec -it [propensity-container-id] /bin/bash`
`root@:/# sqlx migrate run`
`root@:/# exit`
`> docker kill [propensity-container-id]`

`> docker run -it -d --network propensity-network -p 8000:8000 address-propensity`

`> docker exec -it [container-id] /bin/bash`


`root@docker-desktop:/app# RUST_LOG=warn ./loader -s resources/secrets.yaml property resources/data/core_property_data.csv`
`root@docker-desktop:/app# RUST_LOG=warn ./loader -s resources/secrets.yaml propensity resources/data/propensity_scores.csv`

`root@docker-desktop:/app# RUST_LOG=info ./server -s resources/secrets.yaml`

