+++
type = "post"
status = "published"
date = "2020-08-01"
readingtime = 17

slug = "lxd-lxc:"
title = "Next-Gen system container management"
thumbnail = "thumbnail.png"
foot = "God is a comic playing to an audience that's afraid to laugh - Voltaire"
description= ""
categories = ["LINUX"]
series = ["CONTAINERS"]
part="1"
tags = ["container","wm","lxd", "lxc", "namespace", "cgrpups" ]
credits = [
    "https://linuxacademy.com/course/lxc-lxd-deep-dive/",
    "https://discuss.linuxcontainers.org/t/running-virtual-machines-with-lxd-4-0/7519",
    "https://blog.simos.info/how-to-use-virtual-machines-in-lxd/",
    "https://wiki.archlinux.org/index.php/Chroot",
    "https://github.com/bresilla/tent",
    "https://wiki.archlinux.org/index.php/Linux_Containers",
    "https://www.techrepublic.com/article/how-to-use-lxd-to-deploy-containers/",
    "https://www.linode.com/docs/applications/containers/beginners-guide-to-lxd-reverse-proxy/"
]


punchline = "Software deployment has been a major problem for decades. On the client side and the server side too. To ease things, containers are in the process of revolutionizing this. Containers integrate smoothly into DevOps; streamlining and stabilizing the move from source code in client side to deployable assets in server side."

[style]
    accent = "#d78700"
    theme = "dark"
+++

# The why?

So, lately i've been deeply involved with \*nix's namespaces and cgrpups. I am trying to build ths tool called: [tent](https://github.com/bresilla/tent), which basically at high level is a [chroot](https://wiki.archlinux.org/index.php/Chroot) clone but with modern flare. So it has some automatic mounting points, user directora manupilations etc...

Along the way, i got to learn quite a lot how linux handles containers, especially with ***liblxc***. The way i was going with it was by trying to understand how Canonical's `lxd` (`lxc`) works. And as usual, in order to remeber what i learned, i decided to document it, so here it goes.


# Linux Containers
Linux containers (LXC) is an operating-system-level virtualization method for running multiple isolated Linux systems (containers) on a single control host (LXC host). It does not provide a virtual machine, but rather provides a virtual environment that has its own CPU, memory, block I/O, network, etc. space and the resource control mechanism. This is provided by the namespaces and cgroups features in the Linux kernel on the LXC host. It is similar to a chroot, but offers much more isolation.

# LXD/LXC
LXD (pronounced “Lex-Dee”) is a system container manager build on top of Linux Containers (LXC). The goal of LXD is to provide an experience similar to a virtual machine but through containerization rather than hardware virtualization. Compared to Docker for delivering applications, LXD offers nearly full operating-system functionality with additional features such as snapshots, live migrations, and storage management. It also offers a simple REST API, network and storage management, project views and easy clustering to dozen of hosts...

{{< image url="containers.png" border="1" width="30" padding="15" >}} LXD Logo. {{< /image >}}

LXD is an open source system container manager, developed by the team behind LXC and supportet by Canonical, written in Go and that's been around for quite some time now. Containers are created from images with prebuilt images available for most Linux distributions. Multiple hosts can easily be clustered together to form one large virtual host, exposing the exact same API as a single host would. Storage pools and networks can also be created and managed through LXD and resources can be segmented into projects.

# LXC vs siblings

Compared to other container syblings, LXD offers plenty of admin-appealing features, such as:

    - Secured using unprivileged containers, resource restrictions and much more
    - Scalable up to thousands of compute nodes
    - Intuitive with a straightforward CLI
    - Numerous images from which to pull
    - Cross-host container and image transfer support
    - Advanced resource control for cpu, memory, network I/O, block I/O, disk usage and kernel resources
    - Device passthrough for USB, GPU, unix character and block devices, NICs, disks and paths
    - Network management for bridge creation and configuration, cross-host tunnels, and more
    - Storage management support for multiple storage backends, storage pools and storage volumes
    - Uses LXC through liblxc and its Go binding to create and manage the containers

# Installation
The first thing to be done is the installation of LXD. As this tool is found in the standard repositories, it can be installed from the package manager of your distro:
```
arch:        pacman -S lxd
debian:      apt install lxd
fedora:      dnf install lxd
alpine:      apk add lxd
```

Once installed, you'll need to add your user to the lxd group and log-out and back in:

{{< command >}}
sudo usermod -aG lxd $USER
{{< /command >}}


# Configutation

To go through the initial LXD configuration, run:

{{< command >}}
sudo lxd init
{{< /command >}}

You will then be asked a number of questions. Unless you need a specific configuration for your setup, the defaults should work just fine. Those questions are:

```
Would you like to use LXD clustering? (yes/no) [default=no]: no
Do you want to configure a new storage pool? (yes/no) [default=yes]: yes
Name of the new storage pool [default=default]: default
Name of the storage backend to use (btrfs, ceph, dir, lvm, zfs) [default=zfs]: zfs
Create a new ZFS pool? (yes/no) [default=yes]: yes
Would you like to use an existing block device? (yes/no) [default=no]: no
Size in GB of the new loop device (1GB minimum) [default=15GB]: 15GB
Would you like to connect to a MAAS server? (yes/no) [default=no]: no
Would you like to create a new local network bridge? (yes/no) [default=yes]:
What should the new bridge be called? [default=lxdbr0]:
What IPv4 address should be used? (CIDR subnet notation, “auto” or “none”) [default=auto]:
What IPv6 address should be used? (CIDR subnet notation, “auto” or “none”) [default=auto]:
Would you like LXD to be available over the network? (yes/no) [default=no]:
Would you like stale cached images to be updated automatically? (yes/no) [default=yes]
Would you like a YAML "lxd init" preseed to be printed? (yes/no) [default=no]: yes
```

Verify that the lxc client is talking to the LXD daemon: `lxc list` and you'lle get an output like:
```
+------+-------+------+------+------+-----------+
| NAME | STATE | IPV4 | IPV6 | TYPE | SNAPSHOTS |
+------+-------+------+------+------+-----------+
```

## Managing containers

The syntax is as follows to create your first container:

{{< command >}}
lxc launch images:{distro}/{version}/{arch} {container-name-here}
{{< /command >}}

The easiest way is to use a built-in remote image server.

You can get a list of built-in image servers with:

{{< command >}}
lxc remote list
{{< /command >}}

and the output will be:
```
+-----------------+------------------------------------------+---------------+-------------+--------+--------+
|      NAME       |                   URL                    |   PROTOCOL    |  AUTH TYPE  | PUBLIC | STATIC |
+-----------------+------------------------------------------+---------------+-------------+--------+--------+
| images          | https://images.linuxcontainers.org       | simplestreams | none        | YES    | NO     |
+-----------------+------------------------------------------+---------------+-------------+--------+--------+
| local (default) | unix://                                  | lxd           | file access | NO     | YES    |
+-----------------+------------------------------------------+---------------+-------------+--------+--------+
| ubuntu          | https://cloud-images.ubuntu.com/releases | simplestreams | none        | YES    | YES    |
+-----------------+------------------------------------------+---------------+-------------+--------+--------+
| ubuntu-daily    | https://cloud-images.ubuntu.com/daily    | simplestreams | none        | YES    | YES    |
+-----------------+------------------------------------------+---------------+-------------+--------+--------+
```
The one we are interested is the first column, `images`, so to list all available images for various Linux distro, notice the `:` at the end of the command:

{{< command >}}
lxc image list images:
{{< /command >}}

Each image contains a basic operating system (for example a linux distribution) and some other LXD-related information.


Let's say you want to launch a container using the Ubuntu image:

{{< command >}}
lxc launch ubuntu CONTAINER_NAME
{{< /command >}}

Where `CONTAINER_NAME` is the name assigned to your container. To get the name of the newly created contaners, list all containers with `lxc list`.

If it already exists then we start it with:

{{< command >}}
lxc start CONTAINER_NAME
{{< /command >}}

### Accessing the Container

If you want to access the shell of your new container (so you can start developing with it), issue the command:

{{< command >}}
lxc exec CONTAINER_NAME -- /bin/bash
{{< /command >}}

We're not just limited using lxc exec as a method of dropping into a container, however. Should we need to run a one-off command, we can do so directly:

{{< command >}}
lxc exec CONTAINER_NAME -- apt install neovim -y
{{< /command >}}

Do note, however, that chaining commands with `&&` or `|` operands does take some small tweaking of the command, using double quotes `"`.
{{< command >}}
lxc exec CONTAINER_NAME -- "apt update && apt install neovim -y"
{{< /command >}}


### Stop or delete the Container

To stop the container, issue the commands:
{{< command >}}
lxc stop CONTAINER_NAME
{{< /command >}}

Or if you want to delete:

{{< command >}}
lxc delete CONTAINER_NAME
{{< /command >}}

## Managing virtual machines
Starting with LXD 4.0, virtual machines are suported natively,and thanks to a built-in agent, they behave almost just like containers 🤯 🤯.

### Creating a VM

Creating a VM is as simple as the following command. It is the standard `lxc launch` command, with the addition of the `--vm` option to create a virtual machine (instead of a system container). We specify the default profile (whichever base configuration you use in your LXD installation). Depending on your computer’s specifications, it takes a few seconds to launch the container/vm, and then less than 10 seconds for the VM to boot up and receive the IP address from your network.

{{< command >}}
lxc launch images:ubuntu/focal ubuntu WM_NAME --vm
{{< /command >}}

or

{{< command >}}
lxc launch images:centos/8 centos WM_NAME --vm
{{< /command >}}

now, running `lxc list` will output:
```
+--------+---------+------+------+-----------------+-----------+
|  NAME  |  STATE  | IPV4 | IPV6 |      TYPE       | SNAPSHOTS |
+--------+---------+------+------+-----------------+-----------+
| ubuntu | RUNNING |      |      | VIRTUAL-MACHINE | 0         |
+--------+---------+------+------+-----------------+-----------+
```

### Accessing the VM

You can see your VM boot with:

{{< command >}}
lxc console WM_NAME
{{< /command >}}

(detach with ctrl+a-q)

Once booted, VMs with the agent built-in will also respond to:

{{< command >}}
lxc exec WM_NAME bash
{{< /command >}}


By installing the LXD agent inside the LXD VM, we can run the usual LXD commands such as `lxc exec`, `lxc file`, etc. Here is how to get a shell, either using the built-in alias `lxc shell`, or `lxc exec` to get a shell with the non-root account of the container images.


### Example: WINDOWS 10
It is possible to build Windows images for LXD, but this process is currently very very manual and involves either a bunch of custom raw.qemu flags to get a temporary graphical console to perform the install or using a separate QEMU process to prepare the image.

The basic steps are:

    - Grab a [Windows ISO image](https://www.microsoft.com/en-ca/software-download/windows10ISO) from Microsoft
    - Grab the [latest virtio drivers](https://fedorapeople.org/groups/virt/virtio-win/direct-downloads/archive-virtio/virtio-win-0.1.173-9/virtio-win.iso) for Windows
    - Create an empty VM with beefier CPU/RAM and SecureBoot disabled: `lxc init win10 --empty --vm -c security.secureboot=false -c limits.cpu=4 -c limits.memory=4GB`
    - Grow its root disk to a reasonable size: `lxc config device override win10 root size=20GB`
    - Enable temporary graphical output and pass the install and drivers media: `echo -n '-device virtio-vga -vnc :1 -drive file=/path/to/Win10_1909_English_x64.iso,index=0,media=cdrom,if=ide -drive file=/path/to/virtio-win-0.1.173.iso,index=1,media=cdrom,if=ide' | lxc config set win10 raw.qemu - `
    - Start the VM: `lxc start win10`
    - Immediately attach using `lxc console win10` and hit ESC until presented with a boot menu
    - Select Boot Manager and then the `QM00001` drive. Then hit ENTER a few times to answer an invisible boot prompt.
    - Use a VNC client to connect to `localhost:1`, you’ll see the installer boot
    - During the installation, load the virtio-scsi driver to access the hard disk
    - Once installed and configured to allow RDP/SSH access, you can remove the various workarounds with: `lxc config unset win10 raw.qemu`
    - Boot the system to confirm it all works, install all the other drivers from the virtio drive, then run the Windows sysprep tool
    - Finally publish your VM as an image with: `lxc publish win10 --alias win10`

# Files
When configuring any container, being able to run commands is only part of the picture. We also need to be able to update and add files. While we could do this by dropping into a shell on our container and working, but LXD also offers us some more streamlined options for working with files on our containers.

## Direct file acces

We can directly edit (using the default `$EDITOR`) a specific file in the container:
{{< command >}}
lxc file edit INSTANCE_NAME/etc/nginx/sites-available/default
{{< /command >}}

Or completely delete file with:
{{< command >}}
lxc file delete INSTANCE_NAME/etc/nginx/sites-available/default
{{< /command >}}

## Copying/Moving from and to container

Copy from an instance to host by using `pull`

{{< command >}}
lxc file pull INSTANCE_NAME/path-in-container path-on-host
{{< /command >}}

Pull a folder with:

{{< command >}}
lxc file pull -r INSTANCE_NAME/path-in-container path-on-host
{{< /command >}}

Copy from host to instance by using `push`

{{< command >}}
lxc file push path-on-host INSTANCE_NAME/path-in-container
{{< /command >}}

Push a folder with:

{{< command >}}
lxc file push -r path-on-host INSTANCE_NAME/path-in-container
{{< /command >}}

# Profiles
When we create a container, where and how it's created and configured is dependent upon its profile. Up until now, we've been using the default profile to assign the storage pool, network, and other information to our containers:
{{< command >}}
lxc profile list
{{< /command >}}
{{< command >}}
lxc profile show default
{{< /command >}}

But we can create specialized profiles to suit our needs.  Let's start by creating a YFS pool:
{{< command >}}
lxc storage create zfs-test zfs source=/dev/nvme1n1
{{< /command >}}

From here, we can use the default profile as a base by creating our zfs-test storage pool via the copy command:
{{< command >}}
lxc profile copy default zfs-test
{{< /command >}}

Let's now update our new profile to use the appropriate storage pool:
{{< command >}}
lxc profile edit zfs-test
{{< /command >}}
```
  root:
    path: /
    pool: zfs-test
    type: disk
```

Now let's launch a new container based on this profile:
{{< command >}}
lxc launch alpine3.10 test2 -p zfs-test
{{< /command >}}

We're not just limited to the existing configurations presented in the default profile, however. We can make a quick `NEW_PROFILE` profile in only a few commands:
{{< command >}}
lxc profile copy default NEW_PROFILE
{{< /command >}}
{{< command >}}
lxc profile set NEW_PROFILE boot.autostart.priority 50
{{< /command >}}
{{< command >}}
lxc profile set NEW_PROFILE boot.stop.priority 50
{{< /command >}}
{{< command >}}
lxc profile set NEW_PROFILE environment.EDITOR vim
{{< /command >}}

To add the profile to our container, we can just use:
{{< command >}}
lxc profile add INSTANCE_NAME NEW_PROFILE
{{< /command >}}

Let's also remove the existing default profile from our `INSTANCE_NAME` container:
{{< command >}}
lxc profile remove INSTANCE_NAME default
{{< /command >}}

As you may have guessed by now, many of the lxc profile commands follow the same conventions as storage and config. Remember, should we ever need a refresher, we can view a full list with:
{{< command >}}
lxc profile --help
{{< /command >}}

# Snapshots
Rarely when working with containers are we working with one-off, individualized configurations. One of the benefits of containers, after all, is the ability to launch many of them very quickly. We don't want these to be stock, unconfigured containers, however. And while we can enroll our containers in a configuration management system, we are more likely to launch our containers based on preconfigured images. These preconfigured images, in turn, are generally created from a configured container. Snapshots can use any naming convention you desire, but is good practise to keep to version numbers. So let's say this snapshot is version 1.0 of our container:
{{< command >}}
lxc snapshot INSTANCE_NAME 1.0
{{< /command >}}

To confirm, we can use the lxc info command, which lists all snapshots associated with the defined container:
{{< command >}}
lxc info INSTANCE_NAME
{{< /command >}}

We can now create a duplicate of this container with lxc copy:
{{< command >}}
lxc copy INSTANCE_NAME/1.0 INSTANCE_NAME_BACKUP
{{< /command >}}

We could also use cp in place of copy. Notice, however, when we view our current list of containers, that our new INSTANCE_NAME_BACKUP container exists, but is not automatically started. We can remedy this with a lxc start:
{{< command >}}
lxc start INSTANCE_NAME_BACKUP
{{< /command >}}

Finally, we can remove any unneeded snapshots with the lxc delete command, the same command we use to remove containers themselves. Instead of listing only the container name, however, we'll list the name and version in the same format we used when copying the snapshot:
{{< command >}}
lxc delete INSTANCE_NAME_BACKUP/1.0
{{< /command >}}

# Other features
You can attach nic and disk devices to them, grow their CPU & RAM, take snapshots, move them between hosts, use them in a cluster, publish them as images, …
VM should pretty much all behave as you would expect a LXD instance, just a bit slower because you’re dealing with a full virtual machine.
