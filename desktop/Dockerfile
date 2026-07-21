FROM kasmweb/ubuntu-jammy-desktop:1.17.0

USER root
RUN sed -i 's/ -sslOnly/ /g' /dockerstartup/vnc_startup.sh && \
    sed -i 's/pem_key.*/&\n    require_ssl: false/' /etc/kasmvnc/kasmvnc.yaml
