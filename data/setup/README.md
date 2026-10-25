# Optional input setup

Only needed if Glide reports that /dev/uinput cannot be opened. Do not run the
whole application with sudo. This setup grants the selected user the ability to
create virtual input devices; membership is security-sensitive.

Review and install the included rule as an administrator:

```
sudo groupadd --system --force glide
sudo usermod -aG glide "$USER"
sudo install -m 0644 70-glide-uinput.rules /etc/udev/rules.d/70-glide-uinput.rules
sudo modprobe uinput
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=misc --sysname-match=uinput
```

Log out and back in to apply the group membership. Some distributions have other
uinput rules; inspect effective permissions if access is still denied. The rule
is not applied automatically.

To undo, remove this rule, remove your user from the glide group, reload udev and
log out/in. Do not remove any rules belonging to other applications.
