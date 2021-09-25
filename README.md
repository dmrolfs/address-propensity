# address-propensity

`> docker build --tag address-propensity --file Dockerfile .`

`> docker run -it -d --network propensity-network -p 8000:8000 address-propensity`

`> docker exec -it [container-id] /bin/bash`

'export APP__DATABASE__HOST=[postgres-container]'

`root@docker-desktop:/app# RUST_LOG=info ./server -s resources/secrets.yaml`

