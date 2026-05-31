//! Pre-built fixture strings for common UFW outputs.
//!
//! Use these in tests to avoid duplicating raw UFW output strings.

/// Inactive UFW status output.
pub const STATUS_INACTIVE: &str = "Status: inactive\n";

/// Active UFW status with a basic SSH rule.
pub const STATUS_ACTIVE_SSH: &str = "\
Status: active
Logging: on (low)
Default: deny (incoming), allow (outgoing), disabled (routed)
New profiles: skip

To                         Action      From
--                         ------      ----
22/tcp                     ALLOW IN    Anywhere
";

/// Active UFW status with SSH, HTTP, and HTTPS rules.
pub const STATUS_ACTIVE_WEB: &str = "\
Status: active
Logging: on (low)
Default: deny (incoming), allow (outgoing), disabled (routed)
New profiles: skip

To                         Action      From
--                         ------      ----
22/tcp                     ALLOW IN    Anywhere
80/tcp                     ALLOW IN    Anywhere
443/tcp                    ALLOW IN    Anywhere
22/tcp (v6)                ALLOW IN    Anywhere (v6)
80/tcp (v6)                ALLOW IN    Anywhere (v6)
443/tcp (v6)               ALLOW IN    Anywhere (v6)
";

/// Numbered status output.
pub const STATUS_NUMBERED: &str = "\
Status: active
     To                         Action      From
[ 1] 22/tcp                     ALLOW IN    Anywhere
[ 2] 80/tcp                     ALLOW IN    Anywhere
[ 3] 443/tcp                    ALLOW IN    Anywhere
";

/// `ufw show listening` output.
pub const SHOW_LISTENING: &str = "\
Listening:
 tcp 0.0.0.0:22
 tcp [::]:22
 tcp 0.0.0.0:80
 udp 0.0.0.0:68
";

/// `ufw show added` output.
pub const SHOW_ADDED: &str = "\
Added user rules (see 'ufw status'):
allow 22/tcp
allow 80/tcp
allow 443/tcp
";

/// Version output.
pub const VERSION: &str = "ufw 0.36.2\nCopyright (C) 2024 Canonical Ltd.\n";

/// An `/etc/default/ufw` config file.
pub const DEFAULT_UFW_CONFIG: &str = "\
# /etc/default/ufw
#

# Set to yes to apply rules to support IPv6 (no means only IPv6 on loopback
# accepted). You will need to 'disable' and then 'enable' the firewall for
# the changes to pick up.
IPV6=yes

# Set the default forward policy to ACCEPT, DROP, or REJECT.  Please note
# that if you change this you will most likely want to adjust your rules
DEFAULT_INPUT_POLICY=\"DROP\"

# Set the default forward policy to ACCEPT, DROP, or REJECT.
DEFAULT_OUTPUT_POLICY=\"ACCEPT\"

# Set the default forward policy to ACCEPT, DROP, or REJECT.
DEFAULT_FORWARD_POLICY=\"DROP\"

# Set the default application policy to ACCEPT, DROP, REJECT or SKIP.
# Please note that setting this to ACCEPT may be a security risk.
DEFAULT_APPLICATION_POLICY=\"SKIP\"

# By default, ufw only touches its own chains. Setting this to 'yes' will
# have ufw manage user chains as well.
MANAGE_BUILTINS=no

# IPT backend - only iptables is supported.
IPT_SYSCTL=/etc/ufw/sysctl.conf
";

/// A managed app profile for a web server.
pub const APP_PROFILE_WEB: &str = "\
# Managed by ufw-kit.
# Do not edit manually unless you also disable this manager.

[WebServer]
title=WebServer
description=Web server profile
ports=80/tcp|443/tcp
";

/// A managed app profile with port ranges.
pub const APP_PROFILE_RANGE: &str = "\
# Managed by ufw-kit.
# Do not edit manually unless you also disable this manager.

[PassiveFTP]
title=PassiveFTP
description=Passive FTP data ports
ports=50000:50100/tcp
";

/// A `ufw.conf` file.
pub const UFW_CONF: &str = "\
# /etc/ufw/ufw.conf
#

# Set to yes to start on boot. If setting this remotely, be sure to add a
# rule to allow your remote connection before starting ufw!
ENABLED=yes

# Set the logging level. Possible values: off, low, medium, high, full
LOGLEVEL=low
";
