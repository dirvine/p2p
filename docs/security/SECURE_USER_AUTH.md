Product Requirements Document (PRD)

Project: Mnemonic-Based Authentication & Quantum-Safe Storage
Version: 0.4 – Dev-ready
Date: 13 Jul 2025

⸻

1. Purpose

Create a developer-friendly security stack for our brand-new Kademlia DHT network that delivers:
	1.	User-controlled secrets – 12-word mnemonics (4096-word list) entered once per device.
	2.	Password-less UX – WebAuthn passkeys for day-to-day unlock.
	3.	Quantum-resistant cryptography – ML-KEM-768 (KEM) & ML-DSA-65 (sig).
	4.	Social recovery – (t/n) Threshold-Dilithium guardians.
	5.	Signed vector-clock updates – tamper-evident state in DHT.
	6.	Algorithm agility – quick swap of primitives via config flag.

⸻

2. Scope (In / Out)

In-Scope	Out-of-Scope
Wallet-style mnemonic onboarding	Traditional passwords, SMS codes
Passkey integration (Web, iOS, Android, Desktop)	Account escrow by company
PQ crypto layer + migration flag	Non-English wordlists v1
Threshold-Dilithium social recovery	Legacy node upgrade path
Signed vector-clock DHT writes	Append-only blob history


⸻

3. Architecture Overview

┌──────────────┐
│ <Mnemonic UI>│ ─┐                      ┌───────────────┐
└──────────────┘  │  Argon2id + SHA3     │ DHT Key = addr│
                  ▼                      ▼               │
       password (144b)          ┌───────────────────┐    │
                                 │Signed VectorClock│◄───┘
┌──────────┐  passkey unlock     │  + Encrypted Blob│
│ Passkey  │───────────────────► │   (AES-256-GCM)  │
└──────────┘                     └───────────────────┘


⸻

4. Functional Requirements

FR-1 Wordlist & Input Component
	•	Wordlist: 4096 English nouns / adjectives; profanity-free.
	•	Unique-prefix length (UPL) ≤ 4. CI fails if > 4.
	•	<MnemonicInput>: auto-complete after UPL chars, hides chars 2-4 to mitigate shoulder-surfing.

FR-2 Secret Derivation

// 12 words → 144 bits raw key material
password  = mnemonicToBytes(words)
key       = Argon2id(password, salt="mnemonic-v1", t=3, m=64MiB)
addr      = sha3_256(password)            // DHT primary key
localKey  = sha3_256("local" + addr)     // filename salt

FR-3 Quantum-Safe Crypto Layer
	•	Key exchange: ML-KEM-768 ⊕ X25519 (hybrid default).
	•	Signatures: ML-DSA-65 (single) + Threshold-Dilithium (t/n) for multi-party.
	•	Runtime flag crypto.scheme = "pq-only" | "pq-hybrid" | "classic" (default pq-hybrid).

FR-4 DHT Storage – Signed Vector Clock

Field	Type	Notes
vc	map<authorID:uint256, counter:uint64>	Logical clock per signer
payload	bytes	User JSON / org update
sig	ML-DSA-65 or Threshold-Dilithium on (vc‖payload)	

	•	Update rule: client increments its own counter vc[self]++, merges any higher counters seen, re-signs, and PUTs to addr.
	•	Conflict resolution: higher lexicographic (vc, sig) wins; devs process all divergent branches.
	•	Integrity: any tamper yields invalid signature.
	•	Sybil defence: replicate to k=20 nodes across AS diversity; optionally relay via Tor/I2P.

FR-5 Passkey Onboarding
	1.	On first device, user types 12 words.
	2.	App stores key inside resident credential (id = addr).
	3.	Subsequent log-ins: WebAuthn → extract key → recompute addr, localKey.

FR-6 Social Recovery (Threshold-Dilithium)
	•	Default t=2, n=3 guardians.
	•	Each guardian holds encrypted share at addr_i = sha3_256(password + guardianID).
	•	Recovery flow:
	1.	User requests re-key.
	2.	Guardians sign new addr' with partials.
	3.	Client combines partials → writes new blob to addr', deletes old.
	•	Fallback: if TD libs unavailable, switch to FROST Schnorr + Dilithium wrapper.

FR-7 SDK Deliverables

Artefact	Language	Target
dht-auth crate	Rust	native + Wasm
@ourorg/dht-auth	TS/JS	Web, React-Native
CLI dht-auth-cli	Rust	ops tooling

Key functions: register(), unlock(), rotate(), recover(), signVcUpdate().

⸻

5. Non-Functional Requirements
	•	Performance: unlock ≤ 500 ms (desktop), ≤ 1.5 s (low-end mobile).
	•	Memory: Argon2 heap ≤ 128 MiB.
	•	Accessibility: WCAG 2.2 AA.
	•	Testing: 90 % unit coverage; nightly lattice-attack KATs.

⸻

6. Security Controls & Mitigations

Risk	Control
Mnemonic prefix leak	Hide chars 2-4; optional 5-char UPL
Device-salt drift	Fixed public salt “mnemonic-v1”
Filename correlation	Append 128-bit random salt
PQ break	Agility flag + annual crypto review
Sybil/eclipse	k=20 replication, AS heuristics
Guardian collusion	Threshold >1; user PQ key encrypts shards
Passkey exhaustion	UI quota monitor + mnemonic fallback


⸻

7. Acceptance Criteria (Happy-Path E2E)
	1.	New user installs app → enters 12 words → receives passkey prompt → unlock succeeds.
	2.	Second device unlocks via passkey only (no mnemonic).
	3.	User updates profile → vector clock counter increments, DHT PUT signed, other peers converge.
	4.	User forgets mnemonic → two guardians approve recovery → new mnemonic works, old one rejected.

⸻

8. Timeline

Milestone	Target
Wordlist freeze	15 Aug 2025
Vector-clock prototype	1 Sep 2025
PQ Threshold PoC	10 Sep 2025
SDK β1	1 Oct 2025
External audit	1 Nov 2025
Launch	1 Dec 2025


⸻

9. Glossary
	•	Mnemonic – human-readable secret (12 words).
	•	Vector clock – per-author logical counter set for causality.
	•	ML-KEM / ML-DSA – NIST lattice-based KEM / signature.
	•	Threshold-Dilithium – (t/n) multi-signature variant of Dilithium.
	•	Passkey – FIDO resident credential.
	•	UPL – Unique-Prefix Length.
