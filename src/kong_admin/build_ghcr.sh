docker build --platform linux/x86_64 -t chisel_ubuntu -f Dockerfile.chisel_ubuntu ..
docker build --platform linux/x86_64 -t ghcr.io/kongzilla-ks/kong_admin -f Dockerfile.kong_admin ..
docker push ghcr.io/kongzilla-ks/kong_admin:latest
