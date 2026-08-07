+++
type = "post"
status = "published"
date = "2020-08-09"
readingtime = 6

slug = "usbip"
title = "Setup a Raspberry Pi as a USB-over-IP server"
thumbnail = "thumbnail.png"
foot = "It's all right letting yourself go as long as you can let yourself back - Mick Jagger"
categories = ["FUSE"]
series = ["USB"]
part="1"
tags = ["usbip", "linux", "hardware", "usb", "network", "FUSE" ]


description = "setup a Raspberry Pi or any Linux server for that matter, to share it's existing usb port through network (USB-over-IP server) and how to access it from another computer (USB-over-IP client)"
punchline = "To share USB devices between computers with their full functionality, USB/IP encapsulates 'USB I/O messages' into TCP/IP payloads and transmits them between computers. USB-over-IP can be useful for virtual machines, for example, that don't have access to the host system's hardware - USB-over-IP allows virtual machines to use remote USB devices."
tldr = "THINGS ARE ABOUT TO GET MESSY"

credits = [
    "https://derushadigital.com/other%20projects/2019/02/19/RPi-USBIP-ZWave.html",
    "http://www.usbmadesimple.co.uk/ums_1.htm",
    "https://beyondlogic.org/usbnutshell/usb3.shtml",
    "https://www.gearprimer.com/technology/usb-connector-types-explained/",
    "https://www.tripplite.com/products/usb-connectivity-types-standards",
]

[style]
    accent = "#04f977"
    theme = "dark"
    width="60%"

+++

# Creators
The Universal Serial Bus (USB) is a specification developed by Compaq, Intel, Microsoft and NEC, joined later by Hewlett-Packard, Lucent and Philips. These companies formed the USB Implementers Forum, Inc as a non-profit corporation to publish the specifications and organise further development in USB.

# Versioning
Despite how long the USB standard has been around, the USB standard hasn’t really gone through a whole lot of major versions although there have been quite a few smaller revisions within each version. Either way, the most important information you’ll probably want to know about a USB version is its maximum theoretical bandwidth/speed.

| Standard        | AKA                                  | Year | Speed    | Connector                                                 | Power |
|-----------------|--------------------------------------|------|----------|-----------------------------------------------------------|-------|
| USB 1.1         | Basic Speed USB                      | 1995 | 12 Mbps  | USB-A USB-B                                               | 2.5W  |
| USB 2.0         | Hi-Speed USB                         | 2000 | 480 Mbps | USB-A USB-B USB Micro-A USB Micro-B USB Mini-A USB Mini-B | 2.5W  |
| USB 3.2 Gen 1   | USB 3.0 USB 3.1 Gen 1 SuperSpeed USB | 2008 | 5 Gbps   | USB-A USB-B USB Micro-B USB-C                             | 4.5W  |
| USB 3.2 Gen 2   | USB 3.0 USB 3.1 Gen 1 SuperSpeed USB | 2013 | 10 Gbps  | USB-A USB-B USB Micro-B USB-C                             | 100W  |
| USB 3.2 Gen 2x2 | USB 3.2 SuperSpeed USB 20Gbps        | 2017 | 20 Gbps  | USB-C                                                     | 100W  |
| USB 4           | USB 4                                | 2019 | 40 Gbps  | USB-C                                                     | 100W  |

{{< hr >}}
# Standards

One{{< sidenote link="http://www.edwardtufte.com/bboard/q-and-a-fetch-msg?msg_id=0000Vt" >}} well this is odd {{< /sidenote >}}

## Host is Master
All communications of this standard are initiated by the host. This means, for example, that there can be no communication directly between USB devices.

A device cannot initiate a transfer, but must wait to be asked to transfer data by the host. The only exception to this is when a device has been put into 'suspend' (a low power state) by the host then the device can signal a 'remote wakeup'.

## On-The-Go
 An extension to the USB specification has been defined, to allow a device to also become a limited role host. USB OTG allows mobile devices such as a smartphone or tablet to act as a host to other USB devices such as flash drives, keyboards and mice. With USB OTG, a mobile device can utilize the functionality of the peripherals while still being able to connect to a computer and present itself as a mass storage device to be used on the computer.

## Other standards
On the most basic level, USB standards simply let a host, such as your computer or tablet, communicate with peripherals and other devices. But as specifications evolve, USB has become more than a mere data interface. Below are the latest USB functions available on many of today's devices. A device may support one or more of these functions:

### WUSB (Wireless)
A short range  high speed radio communications protocol ( 480 Mbit/s up to 3 m and 110 Mbit/s up to 10 ) which seems to aim to compete with Bluetooth and Wi-Fi.

### USB PD (Power Delivery)
Up to 100W of power can be delivered across a single USB cable, eliminating the need for a separate power brick. This is especially useful for peripherals that draw higher power levels, such as hard drives and printers. Not all devices will support USB Power Delivery, however; consult your device's specifications chart or owner's manual if you are uncertain.


### USB Alt Mode
With Alt Mode, USB-C connectors and cables have the ability to transmit both USB data and VGA, DVI, HDMI or DisplayPort video and/or audio. Adapters are available to connect DisplayPort over USB-C to VGA, DVI, HDMI and DisplayPort monitors. DisplayPort Alt Mode does not require the use of drivers, making it plug-and-play.

{{< hr >}}
# USB/IP
USB/IP Project aims to develop a general USB device sharing system over IP network. To share USB devices between computers with their full functionality, USB/IP encapsulates "USB I/O messages" into TCP/IP payloads and transmits them between computers.

The goal here is to create a USB over IP service on a Raspberry Pi, plug the USB radio(s) into the Pi, then place that device on your network somewhere close to the controlled devices. Then Home Assistant can run wherever you like while controlling your devices over an IP link to the radio(s).

## Setting up the USB/IP server
### Server Requirements

- Raspberry Pi running Rasbian (or something like it)
{{< sideimage url="raspberry-pi-side.jpg" >}}
Raspberry Pi and USB ports
{{< /sideimage >}}
- USB dongle to share

### Server Process

SSH to raspbian and execute the following commands:
```
sudo -s
lsusb
```


lsusb should show a list of attached USB devices, here’s what mine looks like:

```
Bus 001 Device 006: ID 10c4:8a2a Cygnal Integrated Products, Inc.
Bus 001 Device 005: ID 0557:2306 ATEN International Co., Ltd
Bus 001 Device 004: ID 0781:5583 SanDisk Corp.
Bus 001 Device 003: ID 0424:ec00 Standard Microsystems Corp. SMSC9512/9514 Fast Ethernet Adapter
Bus 001 Device 002: ID 0424:9514 Standard Microsystems Corp.
Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
```


We’re looking for the device identifier for the USB radio, which in my case is that Cygnal Integrated Products, Inc. device with an ID of 10c4:8a2a. We’ll then setup a systemd service definition that is going to search for that device string and attach it to the USBIPd service. If you’re using a different USB device, change the device ID in the lines below for ExecStartPost and ExecStop

```bash
# Install usbip and setup the kernel module to load at startup
apt-get install usbip
modprobe usbip_host
echo 'usbip_host' >> /etc/modules

# Create a systemd service
vi /lib/systemd/system/usbipd.service
```

Copy and paste the following service definition:

```bash
[Unit]
Description=usbip host daemon
After=network.target

[Service]
Type=forking
ExecStart=/usr/sbin/usbipd -D
ExecStartPost=/bin/sh -c "/usr/sbin/usbip bind --$(/usr/sbin/usbip list -p -l | grep '#usbid=10c4:8a2a#' | cut '-d#' -f1)"
ExecStop=/bin/sh -c "/usr/sbin/usbip unbind --$(/usr/sbin/usbip list -p -l | grep '#usbid=10c4:8a2a#' | cut '-d#' -f1`); killall usbipd"

[Install]
WantedBy=multi-user.target
```

Save that file, then run the following commands in your shell:

```bash
# reload systemd, enable, then start the service
sudo systemctl --system daemon-reload
sudo systemctl enable usbipd.service
sudo systemctl start usbipd.service
```

## Setting up the USB/IP client
### Client Requirements

- Linux server/desktop (Tested on Ubuntu server 17.04. There are some Windows builds but I couldn’t get any of them to work reliably under Win10/Server 2016)
- IP address of your RPi running as a server. Here I’m using 192.168.0.10.

### Client Process

SSH to the Linux server and execute the following commands:

```bash
sudo -s
apt-get install linux-tools-generic -y
modprobe vhci-hcd
echo 'vhci-hcd' >> /etc/modules
```

Much like we did on the server, we’re going to need to modify the ExecStart and ExecStop lines below to search for the correct USB device ID that’s being presented by your USB/IP server. Likewise, change the IP 192.168.0.10 to match your RPi USB server.

```bash
vi /lib/systemd/system/usbip.service
```

Copy and paste the following service definition:

```bash
[Unit]
Description=usbip client
After=network.target

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/bin/sh -c "/usr/lib/linux-tools/$(uname -r)/usbip attach -r 192.168.0.10 -b $(/usr/lib/linux-tools/$(uname -r)/usbip list -r 192.168.0.10 | grep '10c4:8a2a' | cut -d: -f1)"
ExecStop=/bin/sh -c "/usr/lib/linux-tools/$(uname -r)/usbip detach --port=$(/usr/lib/linux-tools/$(uname -r)/usbip port | grep '<Port in Use>' | sed -E 's/^Port ([0-9][0-9]).*/\1/')"

[Install]
WantedBy=multi-user.target
```

Save that file, then run the following commands in your shell:

```bash
# reload systemd, enable, then start the service
sudo systemctl --system daemon-reload
sudo systemctl enable usbip.service
sudo systemctl start usbip.service
```

You should now be able to access the USB device over the network as if the device was plugged in locally, and you have an auto-starting systemd service to control things.)
