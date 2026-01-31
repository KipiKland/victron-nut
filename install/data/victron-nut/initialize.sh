#!/bin/bash

echo "Run victron-nut initialize script ..."

if [ ! -L /usr/share/ca-certificates/franz-certs ]; then
	echo "Add franz-certs symlink ..."
	ln -s /data/victron-nut/franz-certs /usr/share/ca-certificates/franz-certs
	update-ca-certificates
fi

if ! grep -q "franz-certs/reiter-edv-ca.crt" /etc/ca-certificates.conf; then
	echo "Add reiter-edv-ca to ca-certificates ..."
	echo "franz-certs/reiter-edv-ca.crt" >> /etc/ca-certificates.conf
	update-ca-certificates
fi

if [ ! -L /service/victron-nut ]; then
	echo "Add victron-nut service ..."
	ln -s /data/victron-nut/service /service/victron-nut
fi
