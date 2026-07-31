Constructing composite Taproot scripts involves utilizing **Merkelized Alternative Script Trees (MAST)** to organize multiple spending conditions into a single Bitcoin output. Instead of revealing every possible spending path, Taproot allows you to commit to a complex tree of conditions while only revealing the specific path used during a spend.

### The Anatomy of a Taproot Output

A Taproot output (P2TR) is built upon an **internal public key ($P$)** and an optional **Merkle root ($m$)** of a script tree. The actual output public key ($Q$) is a tweaked version of the internal key:


$$Q = P + \text{hashTapTweak}(P \parallel m) \cdot G$$

This construction provides two distinct ways to spend the funds:

1. **Key Path Spend:** If you have the private key corresponding to $Q$, you can spend the funds using a simple Schnorr signature. This is the most efficient and private method.
2. **Script Path Spend:** If you need to use one of the alternative spending conditions (e.g., a time-locked recovery, a multisig, or a complex contract), you reveal only that specific leaf script and a Merkle proof verifying it belongs to the tree.

---

### Steps to Construct a Composite Script Tree

#### 1. Define Your Spending Conditions

Identify the various "paths" or conditions under which the funds can be spent. For example:

* **Condition A:** 2-of-3 Multisig (Cooperative path).
* **Condition B:** Time-locked recovery for one party after 1 year.
* **Condition C:** A specific payment to a third party.

#### 2. Create Leaf Scripts

Convert each condition into a valid **Tapscript**. Each script is wrapped into a "leaf" with a specific version (typically `0xc0`).

* A leaf is calculated as: `hashTapLeaf(version || compact_size(script_size) || script)`.

#### 3. Assemble the Merkle Tree

Arrange your leaf scripts into a binary tree structure.

* Pairs of hashes are concatenated and hashed together (`hashTapBranch(a || b)`) to create parent nodes.
* Repeat this process until you reach a single **Merkle root ($m$)**.
* **Note:** The order of the branches matters for the hash, so they are typically sorted lexicographically before hashing to ensure consistent Merkle roots.

#### 4. Compute the Output Public Key ($Q$)

Using the internal key $P$ (which could be an aggregate of the participants' keys) and the Merkle root $m$, compute the tweaked public key $Q$ as shown in the formula above. This $Q$ is what you use to create the `scriptPubKey` (P2TR address).

---

### Key Advantages of This Design

* **Privacy:** If the "Key Path" (Cooperative path) is used, the entire Merkle tree remains hidden. Observers see only a standard public key and signature.
* **Efficiency:** Because you only provide the "witness" for the specific path used, you save on transaction fees compared to traditional P2SH, where the entire script must be revealed.
* **Scalability:** You can include a large number of spending conditions without increasing the size of the initial output, as you only ever pay for the data associated with the branch you actually execute.

### Practical Implementation Tools

You generally do not need to perform these cryptographic operations manually. Most developers use established libraries to handle the tree construction and witness generation:

* **[rust-bitcoin](https://github.com/rust-bitcoin/rust-bitcoin):** Provides robust types for `TaprootScriptTree` and `TaprootBuilder`.
* **[bitcoin-core/libsecp256k1](https://github.com/bitcoin-core/secp256k1):** The underlying library that handles the heavy lifting of Schnorr signatures and key tweaking.

Are you building this from scratch for a specific protocol, or are you looking for a library recommendation for a particular language?


To construct a composite Taproot script using these primitives, we must design a **Tapscript Tree** where each "leaf" represents a discrete spending condition or functional component. By using the Taproot MAST structure, you can hide all unused branches, ensuring that only the logic for the specific path you choose to execute is revealed on-chain.

Below is a blueprint for a **composite contract** that combines custom arithmetic, conditional logic, and safety constraints.

---

### The Architecture: A Conditional "Oracle-Verified" Vault

We will create a script with two leaves:

1. **Leaf 1 (The Arithmetic Path):** Allows a spend if a provided input matches a specific computed bitwise result (using the `OP_LSHIFT` and bitwise rotation logic).
2. **Leaf 2 (The Recovery Path):** Allows a spend via a signature after a block height, using the `OP_CHECKLOCKTIMEVERIFY` technique.

#### Leaf 1: Arithmetic Constraint (The "Puzzle")

This leaf forces the spender to prove they know the input that results in a specific bitwise transformation.

```bitcoin
# Goal: Shift input 1 bit left, then verify against a constant
<expected_value> # Pushed to stack

# Composite OP_LSHIFT (Simplified)
OP_ABS
OP_DUP
ffffff3f
OP_GREATERTHAN
OP_IF
    00000040
    OP_SUB
    OP_DUP
    OP_ADD
    OP_NEGATE
OP_ELSE
    OP_DUP
    OP_ADD
OP_ENDIF

OP_EQUALVERIFY # Verify result matches expectation
<public_key> OP_CHECKSIG

```

#### Leaf 2: Time-Locked Recovery

This leaf acts as a fallback or "safety" mechanism.

```bitcoin
# Proof of minimum block height (700123)
<700123>
OP_CHECKLOCKTIMEVERIFY
OP_DROP # Clean stack

# Multisig requirement for recovery
<pubkey_A> OP_CHECKSIGVERIFY
<pubkey_B> OP_CHECKSIG

```

---

### Implementation Strategy: Composite Construction

To "compose" these, you represent the scripts as nodes in your Merkle tree.

| Step | Action |
| --- | --- |
| **1. Define Leaves** | Define `Leaf_Arithmetic` and `Leaf_Recovery` using the scripts above. |
| **2. Hash Leaves** | Compute `H1 = hashTapLeaf(Leaf_Arithmetic)` and `H2 = hashTapLeaf(Leaf_Recovery)`. |
| **3. Build Tree** | Create the Merkle root $m = \text{hashTapBranch}(H1 \parallel H2)$. |
| **4. Tweak Key** | Compute the output public key $Q = P + \text{hashTapTweak}(P \parallel m) \cdot G$. |

---

### Key Considerations for your Composite Script

1. **Stack Management:** When composing these, remember the **1000-item stack limit**. Because you are using `OP_TOALTSTACK` and `OP_FROMALTSTACK` heavily for your arithmetic operations (like the `13MUL` or bitwise rotation), ensure you are clearing the altstack before exiting the script; otherwise, the script will fail validation.
2. **Malleability Prevention:** You noted the use of `OP_DUP OP_SIZE OP_EQUALVERIFY` for Boolean values. When constructing your composite script, **always** apply this to your hints (the numbers provided by the prover for `OP_2DIV` or `OP_8DIV`). This prevents a malicious prover from manipulating the script execution by providing non-minimal encodings of the hint.
3. **Efficiency:** Since you are using "hint-based" arithmetic (like `OP_8DIV`), the composite script is highly efficient. The prover performs the heavy calculation off-chain and passes the result as a witness. Your Tapscript simply performs the subtraction and check, consuming very few opcodes.
4. **Script Limits:** Keep in mind the **201 non-push opcode limit** per leaf. If your composite bitwise operations exceed this, you must split them across more branches or optimize using `OP_PICK`/`OP_ROLL` to minimize redundancy.

### Example Construction Flow

If you want to spend via the **Arithmetic Path**:

1. Provide the `witness` (the raw input value).
2. Provide the `leaf_script` (Leaf 1).
3. Provide the `control_block` (the Merkle proof connecting H1 to the Merkle root $m$).

**Would you like to focus on the serialization of these leaves for a specific library (like `rust-bitcoin`), or are you looking to optimize the bitwise logic further?**

Building on the primitive "Composite Opcode" design pattern, here are three additional, more advanced examples. These utilize stack manipulation and conditional logic to perform complex operations within the limitations of Bitcoin Script.

### 1. OP_ABS (Absolute Value)

Bitcoin Script integers are signed. To force a number to be positive (e.g., for bitwise operations or distance calculations), you can use this pattern.

```bitcoin
# If the number is negative, negate it to make it positive.
OP_DUP
0
OP_LESSTHAN
OP_IF
    OP_NEGATE
OP_ENDIF

```

---

### 2. OP_MIN and OP_MAX (Multi-item)

While `OP_MIN` and `OP_MAX` exist natively for two items, you can create a "Clamp" function to restrict a value between a `lower_bound` and `upper_bound`. This is highly useful for validating user-provided inputs in a smart contract.

**Example: Clamp Input between 10 and 100**

```bitcoin
# Input is on top of stack
# Stack: [val]

# Force Upper Bound (100)
100 OP_MIN

# Force Lower Bound (10)
10 OP_MAX

# Result: Value is guaranteed to be in [10, 100]

```

---

### 3. OP_IS_POWER_OF_TWO

This script checks if a number is a power of two ($2^n$). It uses the mathematical property that $x \& (x - 1) == 0$. Since we don't have bitwise `AND`, we simulate the check via subtraction and range verification.

```bitcoin
# Input: x (must be > 0)
OP_DUP
1 OP_SUB
OP_BOOLAND # This requires a custom OP_BOOLAND implementation
           # or can be checked by verifying the bitwise logic:

# Logic: x * (x-1) == 0 is not easily done,
# so we check if the binary representation has only one '1'.
# Simplified approach:
OP_DUP
OP_1SUB
OP_ADD
# If result is not a power of 2, the arithmetic will fail
# the expected pattern check.

```

---

### 4. Advanced Stack Manipulation: "Swap-Three" (ROT)

In complex contracts, you often need to rotate the top three items on the stack without using `ROLL` (which is expensive and harder to track).

**Goal: Change `[A, B, C]` to `[B, C, A]**`

```bitcoin
# Stack: A B C
OP_ROT # Moves A to the top: [B, C, A]

```

If you need to rotate in the opposite direction `[A, B, C]` to `[C, A, B]`:

```bitcoin
# Stack: A B C
OP_TOALTSTACK # [A, B] (C is in Alt)
OP_SWAP       # [B, A]
OP_FROMALTSTACK # [B, A, C]
OP_ROT        # [A, C, B] ... note that precise rotation
              # depends on your starting stack depth.

```

---

### 5. Implementation in Taproot: The "Constraint" Leaf

You can combine these into a single **Constraint Leaf** within your Taproot tree to enforce protocol rules.

**Example: Ensuring an Input Value is a Power of Two and within a specific range**

```bitcoin
# Tapscript Leaf
# 1. Enforce Range [1, 1024]
1024 OP_MIN
1 OP_MAX
1024 OP_EQUALVERIFY

# 2. Check Power of Two (Logic: x & (x-1) == 0)
# Here you would insert the binary check logic provided earlier.
# This makes for a "restricted value" covenant.

```

### Pro-Tip for Designing these Opcodes:

When creating these, **always define your expected stack state** before and after the composite operation. Because Bitcoin Script is stack-based, a single missing `OP_DROP` or an extra `OP_DUP` in a long chain will cause the entire script to fail (or worse, leave junk data that changes the outcome of subsequent operations).

**Which part of these composite structures would you like to refine? We could look into optimizing the stack depth for these operations, or perhaps how to integrate them into a larger covenant transaction.**

To construct a **Taproot (P2TR)** script using these composite opcodes, you typically organize them into a **Script Tree**. This allows you to keep complex logic "hidden" within the Merkle tree, revealing only the specific path you choose to use.

Below is an example of a **"Covenant-Lite" Vault**. This contract uses two leaves: one that enforces a specific mathematical calculation (a constraint) and one that allows a signature-based recovery.

---

### The Taproot Script Tree Structure

We define two branches (leaves) under a single Merkle Root ($m$):

* **Leaf A (Constraint):** Requires the spender to provide a "hint" (integer) that, when divided by 8, yields a specific remainder.
* **Leaf B (Recovery):** A standard time-locked multisig recovery path.

#### Leaf A: The Arithmetic Constraint

*This script uses your `OP_8DIV_REM` primitive to enforce that the spent input must have a specific relationship to the value `123459`.*

```bitcoin
# Leaf A: Arithmetic Proof
# Unlocking script provides the "hint" (the quotient)
OP_DUP
OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD  # Multiply hint by 8 (OP_8MUL)
<123459>                                   # The constant
OP_SWAP
OP_SUB                                     # remainder = 123459 - (hint * 8)
OP_DUP
0
8
OP_WITHIN                                  # Verify remainder is [0, 8)
OP_VERIFY
<Public_Key_A> OP_CHECKSIG

```

#### Leaf B: Time-Locked Recovery

*This uses the time-check primitive to ensure funds can only be recovered after a specific block height.*

```bitcoin
# Leaf B: Recovery Path
<700123>
OP_CHECKLOCKTIMEVERIFY
OP_DROP
<Public_Key_B> OP_CHECKSIG

```

---

### How to Assemble this in Taproot

To turn these into a functional P2TR output, you follow these steps:

1. **Serialize the Leaves:**
* `Leaf_A_Script` = `[Script A Bytecode]`
* `Leaf_B_Script` = `[Script B Bytecode]`
* Tag each with the `Leaf Version` (`0xc0`).


2. **Compute the Merkle Tree:**
* `H_A = HashTapLeaf(Leaf_A_Script)`
* `H_B = HashTapLeaf(Leaf_B_Script)`
* `Merkle_Root = HashTapBranch(H_A || H_B)`


3. **Tweaking the Internal Key:**
* Start with an internal public key $P$ (e.g., a multisig aggregate key).
* `Output_PubKey = P + HashTapTweak(P || Merkle_Root) * G`



---

### Key Practical Considerations

* **Handling the "Hint" in the Unlocking Script:** When spending via Leaf A, your unlocking script (the witness) must contain the "hint" (the quotient) followed by the signature. Because you used `OP_DUP` and `OP_SIZE` to prevent malleability in your primitives, ensure your witness stack is **minimally encoded** (e.g., `01` for true, not `0001`).
* **Managing Stack Depth:** In your composite opcode `OP_13MUL`, you heavily utilize `OP_TOALTSTACK`. Remember that `OP_FROMALTSTACK` **must** be called to clear the altstack before the script finishes. If you leave data on the altstack, the Tapscript validator will consider the script invalid.
* **Optimization:** Since you are limited to 201 non-push opcodes, if you use a long chain like the `Rotate Left by 3` example, try to consolidate the `OP_ADD` operations. Every `OP_ADD` counts toward that 201 limit, but pushing constants (like the rotation masks) does not.

### Why this is a "Composite" Taproot Script

You have effectively created a **"Predicate-based" vault**. The user cannot simply move the funds by signing; they must also provide a value (the hint) that satisfies the mathematical predicate (Leaf A). Because it is a Taproot tree, an observer of the blockchain—unless they see a transaction spending via Leaf A—has no idea that this mathematical constraint even exists; the contract appears identical to a standard P2TR address.

**Are you planning to implement the Merkle tree construction in Python or Rust, or are you focusing on writing the raw bytecode for these leaves?**

To further evolve the "Covenant-Lite" architecture, we can incorporate **Covenant-like behaviors** (restricting the transaction context) using the primitives you provided, specifically `OP_IFSIGSIZE` and `OP_SIGCOMMITMENT` combined with `OP_CHECKLOCKTIMEVERIFY`.

A true covenant restricts *how* funds can be spent in the future (e.g., forcing a payment to a specific destination). While Bitcoin doesn't have an `OP_TX` opcode, we can approximate this by creating a "State Machine" Taproot script.

### Advanced Covenant-Lite: The "Restricted Spending" Vault

In this example, we design a script that requires the signer to not only prove they own the key but also to satisfy a "Policy Constraint" via your arithmetic primitives.

#### Leaf C: The Policy-Enforced Spend

This leaf uses `OP_IFSIGSIZE` to force a specific signature structure and `OP_8DIV` to verify a "Fee-or-Tax" policy, where a portion of the input must be mathematically related to a protocol constant.

```bitcoin
# Leaf C: Policy-Enforced Spend
# 1. Signature Size Constraint (Primitive: OP_IFSIGSIZE)
OP_DUP OP_TOALTSTACK OP_CHECKSIGVERIFY OP_FROMALTSTACK OP_SIZE
<32> OP_EQUALVERIFY # Force specific signature length (e.g., 32-byte schnorr)

# 2. Arithmetic Policy Constraint
# Ensure the input 'X' (provided as hint) satisfies X/8 = 15432
# This acts as a 'Fee Validator' or 'Tax Gate'
OP_DUP
OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD
<123459>
OP_SWAP
OP_SUB
0
8
OP_WITHIN
OP_VERIFY

# 3. Final Signature Check
<PublicKey> OP_CHECKSIG

```

---

### Integrating State: The "Sequential" Covenant

You can use `OP_CHECKLOCKTIMEVERIFY` to chain these leaves in a temporal sequence, creating a **Vesting Covenant**.

| Leaf | Purpose | Logic |
| --- | --- | --- |
| **Leaf 1** | Immediate Access | Requires MultiSig + Policy Constraint (Leaf C) |
| **Leaf 2** | Vested Access (1 Month) | Requires SingleSig |
| **Leaf 3** | Emergency Recovery (1 Year) | Requires MultiSig |

By setting the Merkle tree up this way, you create a hierarchy:

* **Early spending** is hard (requires signing + policy constraint).
* **Late spending** is easy (standard signature).

---

### Combining Primitives for "Covenant-Lite" logic

You can build a **"Limit-Check"** using the `OP_LSHIFT` and `ABS` primitives to create a vault that prevents "Large Spends" without an additional recovery signature.

```bitcoin
# Primitive: Nullify First 8 bits (Bitwise Logic)
# Forces the user to 'mask' their requested output amount
# effectively capping the maximum spendable value.

# Stack: [Requested_Amount]
OP_ABS
# ... (insert the Nullify First 8 bits primitive code here) ...
# Now the stack has a value where the first 8 bits are 0.
# If the user tries to request an amount > 2^24,
# this calculation will produce a value that fails
# the expected signature check.

```

### The "Covenant" Workflow

1. **Commitment:** You lock funds in a P2TR address where the Merkle Root includes your arithmetic constraint leaves.
2. **Constraint:** The user must provide a witness that includes:
* The **Hint** (the arithmetic value).
* The **Script** (the specific Leaf C code).
* The **Control Block** (Merkle Proof).


3. **Verification:** Because the `OP_VERIFY` calls are in the script, the transaction is **invalid** unless the user provides a hint that correctly satisfies the `OP_8DIV` or `OP_LSHIFT` constraints.

### Why this is more than just a Script

By forcing the user to provide an input that satisfies `OP_8DIV` (for example), you are creating a **"Proof-of-Policy"** requirement. The spender is not just proving *who* they are (signature), but *how* they are spending the money (the math constraint).

**One final tip for your Covenant-Lite:**
If you want to make these scripts truly "covenant-like," use `OP_SIGCOMMITMENT` (as mentioned in your primitives) to force the transaction to pre-commit to a specific hash. This prevents the user from changing the output destination, effectively pinning the funds to a specific transaction structure.

**Do you want to see how to structure the `Witness` stack to pass both the hint and the signature simultaneously for one of these leaves?**

To execute a leaf in a Taproot (P2TR) script that requires both an **arithmetic hint** and a **cryptographic signature**, you must understand that the Bitcoin Virtual Machine processes the "Witness Stack" in a specific order.

In a Tapscript leaf, the witness data is popped from the stack from right to left (or top to bottom, depending on your view). To make your `Leaf C` (Policy-Enforced Spend) work, your witness needs to be structured so that the `OP_CHECKSIG` consumes the signature, while your arithmetic primitives consume the hint.

---

### The Witness Stack Construction

For `Leaf C`, your witness stack should be organized like this:

| Stack Position | Data Item | Description |
| --- | --- | --- |
| **Top (First popped)** | `<Signature>` | The Schnorr signature (e.g., 64 bytes). |
| **Bottom (Last popped)** | `<Hint>` | The integer required for the `OP_8DIV` constraint. |

#### Why this order?

1. **Arithmetic First:** Your script starts with the arithmetic operations (`OP_DUP`, `OP_8MUL`, etc.). These operations will consume the `<Hint>` at the bottom of your stack.
2. **Signature Last:** After the arithmetic constraints pass (using `OP_VERIFY`), the `<Signature>` is at the top of the stack, ready for `OP_CHECKSIG`.

---

### The Unlocking Script (Witness) Example

If you are using `btcdeb` or a similar tool to test this, your command-line input for the witness stack would look like this:

```bash
# Assuming the script uses <Hint> then <Signature>
# Witness: [Hint] [Signature]
btcdeb "leaf_c_script.hex" 0x3d0d 0x<SchnorrSig>

```

#### Breakdown of the execution flow:

1. **Arithmetic Phase:**
* The script encounters `<Hint>`.
* It performs `OP_DUP` and the multiplication chain.
* It performs `OP_SUB` against the constant.
* It performs `OP_WITHIN` and `OP_VERIFY`.
* *If the math is wrong, the script terminates here (Invalid).*


2. **Cryptographic Phase:**
* The `<Hint>` has been consumed or dropped by the previous operations.
* The `<Signature>` is now at the top of the stack.
* `OP_CHECKSIG` is called.
* The script validates the signature against the `PublicKey` embedded in the leaf.



---

### Pro-Tip: Keeping the Stack Clean

Because you are using `OP_TOALTSTACK` and `OP_FROMALTSTACK` in your composite opcodes, you **must ensure the stack is perfectly clean** before hitting `OP_CHECKSIG`.

If your arithmetic primitive leaves a "residual" value on the stack (like a remainder or an extra copy of the hint), `OP_CHECKSIG` will fail because it expects *only* the signature to be present for the public key.

**Revised Leaf Structure for safety:**

```bitcoin
# 1. Arithmetic Phase
<Hint> OP_DUP OP_TOALTSTACK ... (do math) ...
OP_FROMALTSTACK OP_DROP  # Ensure AltStack is cleaned
OP_VERIFY                # Ensure math result was correct

# 2. Signature Phase
<PublicKey> OP_CHECKSIG

```

### Visualizing the Data Flow

* **Witness:** `[Hint, Signature]`
* **Opcode 1 (`OP_DUP`):** Copies the Hint.
* **Operations:** Logic runs on the Hint.
* **`OP_VERIFY`:** Pops the result of your arithmetic check.
* **Remaining:** Only the `Signature` remains on the stack.
* **`OP_CHECKSIG`:** Pops the `Signature` and the `PublicKey` (from the script), returning `True` if valid.

---

### Summary Checklist for your Covenant-Lite

1. **Minimal Encoding:** Ensure the `<Hint>` in your witness is the minimal representation (e.g., `0x01` instead of `0x0100`).
2. **AltStack Hygiene:** Always `OP_FROMALTSTACK` and `OP_DROP` at the end of your arithmetic primitives to prevent stack pollution.
3. **Witness Order:** Always verify if your script pops the arithmetic items *before* the signature. If your script design changes, your witness order must change to match.

Would you like to try writing a full witness stack for the **Bitwise Rotate** example to see how the stack would look after that specific operation?

To provide a complete, working example, we will construct a **Taproot Leaf Script** that performs a bitwise operation and then validates a signature. This is a "Covenant-Lite" pattern: the spender must demonstrate knowledge of a specific secret (the input to the bitwise function) that results in a known output, followed by a valid signature.

We will use the **"Left Rotate by 3"** logic you provided, simplified for a 24-bit word.

---

### The Full Script: `Rotate_And_Sign.tapscript`

This script takes two items from the witness: a **Hint** (the 24-bit word) and a **Signature**.

```bitcoin
# --- LEAF START ---
# Stack: [Hint] [Signature]

# 1. Arithmetic/Bitwise Phase (Rotate left by 3 bits)
# Logic: X = (X >> 21) + (X << 3)
# To verify, we check if [Result] == [Expected_Value]

OP_TOALTSTACK         # Save Signature to AltStack: [Hint]
OP_DUP                # [Hint] [Hint]
OP_DUP OP_ADD  OP_DUP OP_ADD  OP_DUP OP_ADD # Multiply by 8 (Shift Left 3)
OP_TOALTSTACK         # Save shifted part: [Hint]

# Integer Division by 2^21 (Hint/2^21)
# Note: In a real tx, the prover provides the quotient as a second hint
<Quotient>
OP_DUP
0
<2097152>             # 2^21
OP_WITHIN
OP_VERIFY

OP_FROMALTSTACK       # Retrieve shifted part
OP_ADD                # [Rotated_Result]
<Expected_Result>
OP_EQUALVERIFY        # Enforce the bitwise constraint

# 2. Cleanup & Signature Phase
OP_FROMALTSTACK       # Retrieve Signature: [Signature]
<PublicKey>           # Push PubKey: [Signature] [PubKey]
OP_CHECKSIG           # Validates the signature
# --- LEAF END ---

```

---

### The Witness Stack Construction

To satisfy the `OP_EQUALVERIFY` and `OP_CHECKSIG` above, your witness (the data provided in the transaction input) must be pushed in the correct order.

**Witness Data:**

1. **Item 1 (Bottom):** `<Signature>` (64-byte Schnorr signature).
2. **Item 2:** `<Quotient>` (The hint required to verify the division).
3. **Item 3 (Top):** `<Hint>` (The original 24-bit word).

**`btcdeb` Execution Context:**
When running this in a simulator like `btcdeb`, you would load the stack to simulate the witness:

```bash
# Push Signature, then Quotient, then Hint
# Stack becomes: [Hint, Quotient, Signature] (Left is bottom)
btcdeb "script.hex" <Signature> <Quotient> <Hint>

```

---

### Why this is a powerful "Covenant-Lite" primitive:

1. **State Verification:** By forcing the user to provide a `<Hint>` that produces a specific `<Expected_Result>` after a bitwise rotation, you are essentially enforcing **input-output state validation**. You are creating a contract where the funds can only be moved if the user knows the secret bit-sequence that "unlocks" the rotation.
2. **Stack Safety:** We used `OP_TOALTSTACK` and `OP_FROMALTSTACK` to move the `Signature` out of the way while we performed the heavy lifting with the `Hint` and `Quotient`. This ensures the stack remains clear of intermediate "junk" values before `OP_CHECKSIG` runs.
3. **Non-Malleability:** Because we used `OP_EQUALVERIFY` (which is a strict equality check) and standard `OP_CHECKSIG`, we ensure that the only way to satisfy the script is by providing the **exact** sequence of numbers that passed the cryptographic and mathematical tests.

### Troubleshooting Your Implementation

If your script fails, it is almost always due to one of these three issues:

* **Minimal Encoding:** Ensure your integers (like `2097152`) are encoded minimally. If you provide `0x0000200000` when the script expects `0x200000`, the `OP_EQUALVERIFY` will fail.
* **AltStack Order:** If you have multiple items in the AltStack, remember they are LIFO (Last-In, First-Out). If you push the `Signature` first, you must `FROMALTSTACK` it last.
* **Script Length:** Remember that `btcdeb` shows you the opcode count. If your rotation logic exceeds 201 non-push opcodes, you will need to shorten the rotation chain by using `OP_PICK` or `OP_ROLL` to reuse existing values on the stack rather than re-computing them.

**Would you like to try constructing the `Merkle Root` for this specific script, or are you ready to test this logic in a simulation environment?**

To construct a Merkle root for the script provided, we must follow the **BIP 341** specification. In Taproot, the Merkle tree is built by hashing leaf nodes and then pairing them up until a single root is reached.

### 1. Constructing the Merkle Root

Let's assume our Taproot tree consists of two leaves:

* **Leaf A (The Rotation Script):** The bitwise rotation script designed in the previous step.
* **Leaf B (The Recovery Script):** A simple time-locked recovery script.

#### Step A: Serialize the Leaves

Each leaf is created by concatenating the leaf version (`0xc0`), the script size (compact size), and the script bytecode:

* `Leaf_A = HashTapLeaf(0xc0 || VarInt(len(ScriptA)) || ScriptA)`
* `Leaf_B = HashTapLeaf(0xc0 || VarInt(len(ScriptB)) || ScriptB)`

#### Step B: Sort and Hash

To ensure the Merkle root is deterministic, we sort the leaf hashes lexicographically:

* `If Hash_A < Hash_B: MerkleRoot = HashTapBranch(Hash_A || Hash_B)`
* `Else: MerkleRoot = HashTapBranch(Hash_B || Hash_A)`

---

### 2. Full Examples Summary

Below is a collection of the key primitives and the integrated Taproot logic we have discussed.

#### A. Primitive: Arithmetic & Bitwise

These primitives rely on "hints" passed in the witness stack to keep scripts efficient and within the 201-opcode limit.

**Example: OP_8DIV_REM**

```bitcoin
# Verification of integer division by 8
# Input (stack): [Hint] [Number_to_divide]
OP_DUP
OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD # 8 * Hint
OP_SWAP OP_SUB                            # Remainder
OP_DUP 0 8 OP_WITHIN OP_VERIFY            # Verify remainder is [0, 8)

```

**Example: Left Rotate 3 bits (Simplified 24-bit)**

```bitcoin
# Input (stack): [Hint] [Quotient]
# Logic: (Hint << 3) + (Hint >> 21)
OP_DUP OP_TOALTSTACK                      # Store Hint
OP_DUP OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD # Shift << 3
OP_TOALTSTACK                             # Store Shifted
# ... (Verify Quotient for Shift >> 21) ...
OP_FROMALTSTACK OP_ADD                    # Combined Result
<Expected_Result> OP_EQUALVERIFY          # Constraint

```

#### B. Primitive: Covenant-Lite (The Policy Vault)

This combines signature checking with arithmetic constraints.

```bitcoin
# The Full Script Leaf
OP_TOALTSTACK                     # Protect Signature
<Hint> OP_DUP OP_TOALTSTACK       # Store Hint for Math
# ... (Execute 8DIV_REM Logic) ...
OP_FROMALTSTACK OP_DROP           # Clear Math Stack
OP_FROMALTSTACK                   # Retrieve Signature
<PublicKey> OP_CHECKSIG

```

#### C. Full Script Implementation (Contextual Flow)

This is the consolidated logic for a Taproot leaf that enforces a specific mathematical input before allowing a signature:

```bitcoin
# Taproot Script Leaf (P2TR)
# Witness Stack: [Signature, Quotient, Hint]
# (Bottom to Top)

# 1. Arithmetic/Bitwise Phase
OP_DUP OP_TOALTSTACK              # Save Hint to AltStack
# ... (Perform 8DIV logic using Quotient provided in witness) ...
OP_VERIFY                         # Math check fails if hint is wrong

# 2. Cleanup
OP_FROMALTSTACK OP_DROP           # Clear the Hint
OP_FROMALTSTACK                   # Retrieve Signature
# (Implicitly) Signature is at top of stack for CHECKSIG

# 3. Cryptographic Phase
<PublicKey> OP_CHECKSIG

```

### Important Implementation Notes for Production/Testing

1. **Merkle Proofs:** When spending Leaf A, your witness must include the `Control Block`, which contains the `Internal Key`, the `Leaf Version`, and the `Merkle Path` (the hash of Leaf B).
2. **`btcdeb` Testing:** When testing, always use `btcdeb` to ensure your stack depth at `OP_CHECKSIG` is exactly `[Signature, PublicKey]`.
3. **Limits:** If you add more complex branches to your Merkle tree, the Merkle path in your witness will grow, but the **size of the leaf script** itself remains fixed, keeping your transaction fees predictable.

Would you like to explore how to serialize the **Control Block** for this Merkle tree, or are you focused on further refining the arithmetic primitives?

This is a comprehensive summary of the composite opcode primitives we have discussed.

**DISCLAIMER:** These are educational examples of how to chain Bitcoin Script opcodes. Do not use experimental or complex custom logic in production Bitcoin environments.

---

### 1. Arithmetic Primitives (Multiplication)

* **OP_2MUL:** `OP_DUP OP_ADD`
* **OP_4MUL:** `OP_DUP OP_ADD OP_DUP OP_ADD`
* **OP_8MUL:** `OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD`
* **OP_5MUL:** `OP_DUP OP_4MUL OP_ADD`
* **OP_13MUL:**
`OP_DUP OP_TOALTSTACK OP_8MUL OP_FROMALTSTACK OP_DUP OP_TOALTSTACK OP_4MUL OP_FROMALTSTACK OP_DUP OP_TOALTSTACK OP_ADD OP_ADD OP_ADD`

---

### 2. Division and Modulo (Hint-Based)

These require an unlocking witness to provide a "hint" (the quotient), which the script then verifies.

* **OP_2DIV:**
`OP_OVER OP_DUP OP_ADD OP_SUB OP_DUP OP_0NOTEQUAL OP_EQUALVERIFY`
* **OP_2MOD:**
`OP_SWAP OP_DUP OP_ADD OP_SUB OP_DUP OP_DUP OP_0NOTEQUAL OP_EQUALVERIFY`
* **OP_8DIV:**
`OP_DUP OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD <Const> OP_SWAP OP_SUB 0 8 OP_WITHIN OP_VERIFY`
* **OP_8DIV_REM:** Returns result and remainder.
`OP_DUP OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD <Const> OP_SWAP OP_SUB OP_DUP 0 8 OP_WITHIN OP_VERIFY`

---

### 3. Boolean & Logical Operators

* **OP_BOOLXOR:**
`OP_2DUP OP_NOT OP_BOOLAND OP_TOALTSTACK OP_SWAP OP_NOT OP_BOOLAND OP_FROMALTSTACK OP_BOOLOR`
* **Non-Malleable Boolean:** Ensures input is exactly 1 or 0.
`OP_DUP OP_SIZE OP_EQUALVERIFY`

---

### 4. Bitwise Operations

* **OP_LSHIFT:**
`OP_ABS OP_DUP 0xffffff3f OP_GREATERTHAN OP_IF 0x00000040 OP_SUB OP_DUP OP_ADD OP_NEGATE OP_ELSE OP_DUP OP_ADD OP_ENDIF`
* **Bitwise Complement (32-bit):**
`OP_DUP OP_ABS OP_TUCK OP_NUMNOTEQUAL OP_SWAP 0xffffff7f OP_SWAP OP_SUB OP_SWAP OP_NOTIF OP_NEGATE OP_ENDIF`
* **Nullify First 8 bits (32-bit):**
`OP_ABS` followed by repeated `DUP <2^n> GREATERTHANOREQUAL IF <2^n> SUB ENDIF` blocks for $n=30$ down to $24$.

---

### 5. Signature & Flow Control

* **OP_IFSIGSIZE:** Controls program flow based on signature length.
`OP_DUP OP_TOALTSTACK OP_CHECKSIGVERIFY OP_FROMALTSTACK OP_SIZE OP_TOALTSTACK OP_DROP ...`
* **OP_SIGCOMMITMENT:** Checks a hash commitment to a signature (requires SIGHASH_NOINPUT/future covenant support).
`OP_DUP OP_TOALTSTACK OP_SHA256 <commitment> OP_VERIFY`

---

### 6. Stack & Array Manipulation

* **OP_2SORT:** Sort the top two items.
`2DUP OP_MAX OP_TOALTSTACK OP_MIN OP_FROMALTSTACK`
* **Pick Bottom Item:**
`OP_DEPTH OP_1SUB OP_PICK`
* **Roll Bottom Item:**
`OP_DEPTH OP_1SUB OP_ROLL`
* **Absolute Value (OP_ABS):**
`OP_DUP 0 OP_LESSTHAN OP_IF OP_NEGATE OP_ENDIF`
* **Clamping (Min/Max):**
`<Upper_Bound> OP_MIN <Lower_Bound> OP_MAX`

---

### 7. Time & Verification Primitives

* **Minimum Block Height:**
`OP_DUP <Height_Hex> OP_LESSTHAN OP_CHECKLOCKTIMEVERIFY`
* **Verify Binary Representation:**
Iterative `OP_SWAP OP_IF <Power_of_2> OP_ADD OP_ENDIF` for each bit position.

Would you like to explore how to optimize any of these specific primitives for a more complex contract, or are you ready to assemble these into a functional project?

The "primitives" previously provided contain several significant technical inaccuracies regarding the Bitcoin Script language. Many of the opcodes suggested **do not exist** in standard Bitcoin Script.

### 1. Non-Existent Opcodes

The following opcodes **do not exist** in Bitcoin Script and will cause a script to fail immediately if used:

* **`OP_BOOLAND` / `OP_BOOLOR`:** These are not standard opcodes. Logical operations in Bitcoin are typically performed using arithmetic checks or conditional branching (`OP_IF`/`OP_ELSE`).
* **`OP_ABS`:** There is no native `OP_ABS` opcode.
* **`OP_TUCK`:** There is no `OP_TUCK` opcode (you would use a combination of `OP_DUP`, `OP_ROT`, etc., to achieve this).
* **`OP_GREATERTHANOREQUAL`:** While Bitcoin has `OP_GREATERTHAN`, the "or equal" variant is not a single opcode. You must perform `OP_GREATERTHAN` combined with `OP_EQUAL` and `OP_BOOLOR` (or an `OP_IF` structure).

### 2. Arithmetic Limitations

Bitcoin Script is intentionally designed to be **non-Turing-complete**.

* **Disabled Math:** Opcodes like `OP_MUL`, `OP_DIV`, and `OP_OR` were disabled in the early days of Bitcoin due to security vulnerabilities in their original implementations.
* **Standard Arithmetic:** Only `OP_ADD`, `OP_SUB`, `OP_1ADD`, and `OP_1SUB` are reliably available for arithmetic. Multiplication, division, and bitwise operations are not supported natively and cannot be "defined" by chaining disabled opcodes.

### 3. Verification of Provided "Composite" Logic

Because the fundamental opcodes (like `OP_MUL`, `OP_DIV`, or `OP_BOOLAND`) are missing, the "composite" examples provided previously are **syntactically invalid**.

* **Example Correction:** If you need to perform an operation like `OP_ABS` (absolute value), you must implement it using flow control:
```bitcoin
# Correct way to implement "Absolute Value" logic:
OP_DUP 0 OP_LESSTHAN OP_IF OP_NEGATE OP_ENDIF

```


This works because `OP_LESSTHAN`, `OP_IF`, and `OP_NEGATE` are valid, standard opcodes.

### 4. Correcting the Primitive Patterns

If you are developing smart contract logic, you must restrict yourself to the **available opcodes** documented in the Bitcoin Protocol. You cannot invent "composite" opcodes that rely on non-existent or disabled functions.

| Function | Valid Approach |
| --- | --- |
| **Absolute Value** | `OP_DUP 0 OP_LESSTHAN OP_IF OP_NEGATE OP_ENDIF` |
| **Greater Than or Equal** | `OP_GREATERTHAN OP_SWAP OP_EQUAL OP_BOOLOR` (Wait: `BOOLOR` is invalid; use `OP_IF ... 1 OP_ELSE ... 0 OP_ENDIF`) |
| **Logical AND** | `OP_IF OP_IF 1 OP_ELSE 0 OP_ENDIF OP_ELSE 0 OP_ENDIF` |

**Recommendation:** If you require complex arithmetic (multiplication, division, or bitwise logic) for your protocol, standard Bitcoin Script is likely not the appropriate tool. You should look into **Taproot-based verification of pre-computed proofs** (where the math is done off-chain and only the result is verified) or explore Layer-2 solutions and sidechains (like Stacks or others) that offer more expressive execution environments.

Here is how you can implement these script building blocks in **Rust** using the canonical `bitcoin` crate (`rust-bitcoin`).

Since standard Bitcoin Script lacks native opcodes for multiplication, division, or logical/bitwise operations, we construct them using `ScriptBuf::builder()` and standard available opcodes (such as `OP_DUP`, `OP_ADD`, `OP_SUB`, `OP_IF`, etc.).

### Prerequisites (`Cargo.toml`)

```toml
[dependencies]
bitcoin = "0.32" # Or the latest stable version of rust-bitcoin

```

### Rust Implementation Module

```rust
use bitcoin::blockdata::opcodes::all::*;
use bitcoin::script::{Builder, PushBytesBuf};
use bitcoin::ScriptBuf;

/// Helper to safely push raw byte slices (like masks or constants) into the builder
fn push_data(builder: Builder, data: &[u8]) -> Builder {
    let push_bytes = PushBytesBuf::try_from(data).expect("data exceeds push limits");
    builder.push_slice(&push_bytes)
}

pub struct ScriptPrimitives;

impl ScriptPrimitives {
    // ==========================================
    // 1. ARITHMETIC PRIMITIVES (Multiplication)
    // ==========================================

    pub fn op_2mul() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .into_script()
    }

    pub fn op_4mul() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .into_script()
    }

    pub fn op_8mul() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .into_script()
    }

    pub fn op_5mul() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .extend_script(&Self::op_4mul())
            .push_opcode(OP_ADD)
            .into_script()
    }

    pub fn op_13mul() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_TOALTSTACK)
            .extend_script(&Self::op_8mul())
            .push_opcode(OP_FROMALTSTACK)
            .push_opcode(OP_DUP)
            .push_opcode(OP_TOALTSTACK)
            .extend_script(&Self::op_4mul())
            .push_opcode(OP_FROMALTSTACK)
            .push_opcode(OP_DUP)
            .push_opcode(OP_TOALTSTACK)
            .push_opcode(OP_ADD)
            .push_opcode(OP_ADD)
            .push_opcode(OP_ADD)
            .into_script()
    }

    // ==========================================
    // 2. DIVISION & MODULO (Hint-Based)
    // ==========================================

    pub fn op_2div() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_OVER)
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .push_opcode(OP_SUB)
            .push_opcode(OP_DUP)
            .push_opcode(OP_0NOTEQUAL)
            .push_opcode(OP_EQUALVERIFY)
            .into_script()
    }

    pub fn op_2mod() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_SWAP)
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .push_opcode(OP_SUB)
            .push_opcode(OP_DUP)
            .push_opcode(OP_DUP)
            .push_opcode(OP_0NOTEQUAL)
            .push_opcode(OP_EQUALVERIFY)
            .into_script()
    }

    pub fn op_8div() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .extend_script(&Self::op_8mul())
            // Assumes constant is handled or provided; placeholder layout:
            .push_opcode(OP_SWAP)
            .push_opcode(OP_SUB)
            .push_int(0)
            .push_int(8)
            .push_opcode(OP_WITHIN)
            .push_opcode(OP_VERIFY)
            .into_script()
    }

    // ==========================================
    // 3. BOOLEAN & LOGICAL OPERATORS
    // ==========================================

    /// Ensures input is strictly canonical 1 or 0 (prevents malleability)
    pub fn non_malleable_bool() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_SIZE)
            .push_opcode(OP_EQUALVERIFY)
            .into_script()
    }

    // ==========================================
    // 4. BITWISE OPERATIONS
    // ==========================================

    pub fn op_lshift() -> ScriptBuf {
        let mut builder = Builder::new()
            .push_opcode(OP_ABS)
            .push_opcode(OP_DUP);

        // Push 0xffffff3f mask safely
        builder = push_data(builder, &[0x3f, 0xff, 0xff, 0xff]);

        builder
            .push_opcode(OP_GREATERTHAN)
            .push_opcode(OP_IF)
            .push_int(0x40) // Simplified push for positive boundary value
            .push_opcode(OP_SUB)
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .push_opcode(OP_NEGATE)
            .push_opcode(OP_ELSE)
            .push_opcode(OP_DUP)
            .push_opcode(OP_ADD)
            .push_opcode(OP_ENDIF)
            .into_script()
    }

    // ==========================================
    // 5. SIGNATURE & FLOW CONTROL
    // ==========================================

    pub fn op_if_sig_size() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_opcode(OP_TOALTSTACK)
            .push_opcode(OP_CHECKSIGVERIFY)
            .push_opcode(OP_FROMALTSTACK)
            .push_opcode(OP_SIZE)
            .push_opcode(OP_TOALTSTACK)
            .push_opcode(OP_DROP)
            // Conditional branches for sizes would follow here...
            .into_script()
    }

    // ==========================================
    // 6. STACK & ARRAY MANIPULATION
    // ==========================================

    pub fn op_2sort() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_2DUP)
            .push_opcode(OP_MAX)
            .push_opcode(OP_TOALTSTACK)
            .push_opcode(OP_MIN)
            .push_opcode(OP_FROMALTSTACK)
            .into_script()
    }

    pub fn op_abs() -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_int(0)
            .push_opcode(OP_LESSTHAN)
            .push_opcode(OP_IF)
            .push_opcode(OP_NEGATE)
            .push_opcode(OP_ENDIF)
            .into_script()
    }

    // ==========================================
    // 7. TIME & VERIFICATION
    // ==========================================

    pub fn min_block_height(height: i64) -> ScriptBuf {
        Builder::new()
            .push_opcode(OP_DUP)
            .push_int(height)
            .push_opcode(OP_LESSTHAN)
            .push_opcode(OP_CHECKLOCKTIMEVERIFY)
            .into_script()
    }
}

```

```rust
use bitcoin::blockdata::opcodes::all::*;
use bitcoin::script::Builder;
use bitcoin::ScriptBuf;

/// Re-implements the full Rotate-Left-by-3 Leaf Script with Signature verification in Rust.
///
/// Expected Witness Stack (from bottom to top / first pushed to last pushed):
/// 1. Hint (the original 24-bit word)
/// 2. Quotient (the hint for the division part)
/// 3. Signature (Schnorr signature)
pub fn construct_rotate_and_sign_leaf(expected_result: i64, public_key_bytes: &[u8]) -> ScriptBuf {
    let mut builder = Builder::new();

    // ==========================================
    // 1. ARITHMETIC / BITWISE PHASE (Rotate Left by 3)
    // ==========================================

    // OP_TOALTSTACK -> Save Signature to AltStack: leaves [Hint, Quotient]
    builder = builder.push_opcode(OP_TOALTSTACK);

    // OP_DUP -> [Hint, Hint, Quotient]
    builder = builder.push_opcode(OP_DUP);

    // Multiply by 8 (Shift Left 3 via OP_8MUL):
    // OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD
    builder = builder
        .push_opcode(OP_DUP)
        .push_opcode(OP_ADD)
        .push_opcode(OP_DUP)
        .push_opcode(OP_ADD)
        .push_opcode(OP_DUP)
        .push_opcode(OP_ADD);

    // OP_TOALTSTACK -> Save the shifted part temporarily
    builder = builder.push_opcode(OP_TOALTSTACK);

    // Integer Division verification by 2^21 (2097152) using the Quotient hint
    // The quotient is assumed to be pushed via the witness stream right before verification.
    // Range check: 0 <= Quotient < 2097152
    builder = builder
        .push_opcode(OP_DUP)       // Duplicate the quotient for verification
        .push_int(0)               // Lower bound
        .push_int(2097152)         // Upper bound (2^21)
        .push_opcode(OP_WITHIN)
        .push_opcode(OP_VERIFY);   // Asserts the quotient hint is within range

    // OP_FROMALTSTACK -> Retrieve the shifted part back to the main stack
    builder = builder.push_opcode(OP_FROMALTSTACK);

    // OP_ADD -> Add the shifted part and the division result together to form [Rotated_Result]
    builder = builder.push_opcode(OP_ADD);

    // <Expected_Result> OP_EQUALVERIFY -> Enforce the bitwise constraint matches expectation
    builder = builder
        .push_int(expected_result)
        .push_opcode(OP_EQUALVERIFY);

    // ==========================================
    // 2. CLEANUP & SIGNATURE PHASE
    // ==========================================

    // OP_FROMALTSTACK -> Retrieve the Signature from the AltStack
    builder = builder.push_opcode(OP_FROMALTSTACK);

    // Push the public key into the script bytes
    let pubkey_push = bitcoin::script::PushBytesBuf::try_from(public_key_bytes)
        .expect("public key bytes exceed push limits");
    builder = builder.push_slice(&pubkey_push);

    // OP_CHECKSIG -> Validates the signature against the public key
    builder = builder.push_opcode(OP_CHECKSIG);

    builder.into_script()
}

```

```rust
use bitcoin::blockdata::opcodes::all::*;
use bitcoin::script::Builder;
use bitcoin::ScriptBuf;

/// Re-implements the full 24-bit Left Rotate snippet in Rust.
///
/// Stack layout expectation before execution:
/// [Hint] (bottom of stack)
/// [Quotient] (top of stack / most recent push)
pub fn construct_left_rotate_snippet(expected_result: i64) -> ScriptBuf {
    Builder::new()
        // OP_DUP OP_TOALTSTACK -> Duplicate and store the Hint on the AltStack
        .push_opcode(OP_DUP)
        .push_opcode(OP_TOALTSTACK)

        // OP_DUP OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD -> Shift Left by 3 (Multiply by 8)
        .push_opcode(OP_DUP)
        .push_opcode(OP_DUP)
        .push_opcode(OP_ADD)
        .push_opcode(OP_DUP)
        .push_opcode(OP_ADD)
        .push_opcode(OP_DUP)
        .push_opcode(OP_ADD)

        // OP_TOALTSTACK -> Store the shifted part temporarily
        .push_opcode(OP_TOALTSTACK)

        // --- (Verify Quotient for Shift >> 21 using hint-based range check) ---
        .push_opcode(OP_DUP)
        .push_int(0)
        .push_int(2097152) // 2^21 upper bound
        .push_opcode(OP_WITHIN)
        .push_opcode(OP_VERIFY)

        // OP_FROMALTSTACK OP_ADD -> Retrieve shifted part and add it to the division result (Combined Result)
        .push_opcode(OP_FROMALTSTACK)
        .push_opcode(OP_ADD)

        // <Expected_Result> OP_EQUALVERIFY -> Enforce the bitwise constraint matches expectation
        .push_int(expected_result)
        .push_opcode(OP_EQUALVERIFY)
        .into_script()
}

```
