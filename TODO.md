x refactor docker demo to move loader to Database.Dockerfile
x - enable to return main (API server) Dockerfile to minimal definition; i.e., more PROD-like
x - loader executable has different dependencies (e.g., for visualization generation) and since used on a local env, the temporary, fatter database initialization container is a better fit.
- refractor to unify *_loader modules
- batch processing
- - configurable batch sizing
- async processing
- bloom filter cache
- - save cache in tmp or central dir? 
- - tmp assumes run from single point
- - central assumes runs are done individually wrt target
x put orig progress bar back in place
x - better ux
x - docker demo issued resolved such that I think progress was not original issue
- refactor project structure to use cargo workspaces
- - improves compile time
- - improves module organization
