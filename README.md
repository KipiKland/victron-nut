# NUT server for Victron GX

This is a NUT server that can be started on a Victron GX device itself.  
It's written in Rust, is very resistant against errors and consumes almost no resources.

### What it does

It starts a NUT server and uses DBUS to receive inverter and battery data from Victron.  
You can connect your servers with the normal nut-client / nut-monitor to this nut server and they will shutdown when the inverter is in invert-mode and the battery is getting empty.

All rules regarding shutdown and restarting can be specified in the configuration file.  
For restarting, the following two actions are supported:
* restart_wol => Sends a wake on lan signal
* restart_http_request => Sends a specific GET or POST http request

### What victron devices are supported

I developed the software to run on my "USV", because Victron inverter and a battery is much cheaper than a real USV.  
It's optimized for the following hardware:
* Plyontech US5000 Battery
* Victron Multiplus II GX 48/3000/35-32

### Installation

1. Follow victron documentation to get SSH access to your GX device
2. Copy **install/data/victron-nut** folder from this repo to **/data/victron-nut** on the GX device
3. Compile binary (or download compiled binaries from my gitlab - https://git.howaner.de/Howaner/victron-nut/-/artifacts) and copy it to **/data/victron-nut/victron-nut-armv7**
4. Copy example.conf to **/data/victron-nut/victron-nut.conf** and customize it
5. Create /data/rc.local file with the following text:
```
#!/bin/bash
/data/victron-nut/initialize.sh
```
6. Make the files executable
```
chmod +x /data/rc.local
chmod +x /data/victron-nut/victron-nut-armv7
chmod +x /data/victron-nut/start.sh
chmod +x /data/victron-nut/initialize.sh
chmod +x /data/victron-nut/service/run
chmod +x /data/victron-nut/service/log/run
```
7. Restart GX device
