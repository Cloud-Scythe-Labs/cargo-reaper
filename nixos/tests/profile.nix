{ pkgs, name, ... }:
{
  users.users = {
    "${name}" = {
      isNormalUser = true;
      description = name;
      home = "/home/${name}";
      createHome = true;
    };
    root = {
      hashedPassword = "";
      hashedPasswordFile = null;
    };
  };

  # Enable audio via pipewire.
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    jack.enable = true;
  };

  # Necessary for Xvfb
  services.xserver = {
    enable = true;
    # This can be changed to another DM like xfce if a GUI is needed for debugging
    displayManager.startx.enable = true;
  };

  environment.systemPackages = with pkgs; [
    reaper
    xdotool
    xvfb-run
    cargo-reaper
  ];
}
