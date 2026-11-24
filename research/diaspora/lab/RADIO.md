# Run a two-node Weave chain over packet radio

The bench node mines and validates complete Weave waves. Every received Cycle has to prove work against all three preceding frontier Cycles and bind those proofs together. A node advances only after verifying all three Cycles. This profile uses fixed low difficulty, public test labels, and its own local state file. It has no monetary ledger, fork recovery, or checkpoint voting.

## 1. Install the lab

Both machines need Python 3.10 or later and a KISS TNC with a local TCP interface. These examples use Dire Wolf. Install it with your distribution's package manager or follow the [Dire Wolf user guide](https://github.com/wb2osz/direwolf/tree/dev/doc). Run the commands from the repository root on both machines.

```sh
git clone --branch diaspora/integration-lab https://github.com/ya1-oa/diaspora-v1.git
cd diaspora-v1
python3 -m unittest discover -s research/diaspora/lab -v
```

## 2. Connect the two radio endpoints

Configure a local Dire Wolf instance for your audio interface and PTT hardware on each computer. Use the same modem mode at both endpoints. A single-channel 1200-baud AFSK audio bench is a starting point. Begin with a wired audio path or a shielded bench setup. Add these settings to each instance's modem configuration. Substitute that station's configured identifier for `LABA` or `LABB`.

```text
MYCALL LABA
MODEM 1200
KISSPORT 8001
AGWPORT 0
```

Keep the KISS TCP port local using host firewall rules. Start the modem with its configuration, for example `direwolf -c direwolf.conf`. The examples use bench identifiers. For an authorized RF setup, use the station identifiers and settings required for your equipment and radio service. U.S. amateur operation has restrictions on pecuniary communications and messages encoded to obscure their meaning. Use public test labels and check the applicable rules before transmitting. See [47 CFR § 97.113](https://www.law.cornell.edu/cfr/text/47/97.113).

## 3. Mine on A and validate on B

Start with a fresh state filename on each computer. Keep each file private to one running lab command.

On B, start listening first:

```sh
python3 research/diaspora/lab/weave_radio.py --state b.state receive --local LABB --peer LABA --timeout 120
```

On A, mine an example wave and send it:

```sh
python3 research/diaspora/lab/weave_radio.py --state a.state mine "radio test one"
python3 research/diaspora/lab/weave_radio.py --state a.state send --local LABA --peer LABB
```

Both machines should report `height=1` and the same frontier digest. Use `--state a.state status` or `--state b.state status` to inspect either state on restart. To advance another wave, repeat the listen, mine and send steps. You can also switch roles and let B mine the next wave. Have one station mine at a time. Sending queues data at your local TNC. Matching receiver status confirms delivery.

If a receive times out, start it again and resend the same wave with `send --height 1 --local LABA --peer LABB`. For a node several waves behind, send missing waves in order, starting a receive for each wave. A node that has diverged needs a fresh state file and ordered replay from a known bench peer. Preserve the original file for inspection.

## Wire format and limits

KISS port 0 carries direct AX.25 UI frames with PID `0xf0`. DSP1 fragments carry at most 128 payload bytes with a transfer hash, fragment index, count, and total size. The receiver ignores frames outside the configured route and validates the transfer hash before applying a wave. Address fields route frames and provide no authentication.

The `DW01` payload is big-endian. It starts with four ASCII magic bytes, a uint32 wave height, and a 32-byte digest committing to the preceding frontier. Three ordered Cycle records follow. Each contains four uint32 nonces (three strands and one binder), a 32-byte Cycle ID, and a uint8 length followed by a 1–32 byte ASCII test label. Each node independently derives the parents and historical archives from its state. The lab caps history at 128 waves and receive buffering at four incomplete transfers. The receiver waits for a retransmission on loss. The software TCP test verifies two separate node processes and matching saved tips. On-air loss, device compatibility, and modem timing need hardware measurements.
