# Task 008: Passkey Identity Integration

## Overview
Implement full passkey authentication in the Saorsa Tauri application with DHT integration for identity storage and resolution. This enables passwordless authentication using WebAuthn.

## Acceptance Criteria
- [ ] Passkey registration flow implemented
- [ ] Passkey authentication working
- [ ] Identity stored in DHT with three-word address
- [ ] Frontend UI components complete
- [ ] End-to-end tests passing

## Technical Details

### 1. Backend Implementation

#### Tauri Command Handlers
Location: `apps/saorsa/src-tauri/src/commands/identity.rs`

```rust
use tauri::State;
use webauthn_rs::prelude::*;

#[tauri::command]
pub async fn start_passkey_registration(
    username: String,
    passkey_manager: State<'_, PasskeyManager>,
) -> Result<CreationChallengeResponse, String> {
    passkey_manager
        .register_passkey(&username)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn complete_passkey_registration(
    credential: RegisterPublicKeyCredential,
    passkey_manager: State<'_, PasskeyManager>,
) -> Result<Identity, String> {
    passkey_manager
        .finish_registration(&credential)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_passkey_authentication(
    passkey_manager: State<'_, PasskeyManager>,
) -> Result<RequestChallengeResponse, String> {
    passkey_manager
        .start_authentication()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn complete_passkey_authentication(
    credential: PublicKeyCredential,
    passkey_manager: State<'_, PasskeyManager>,
) -> Result<Identity, String> {
    passkey_manager
        .finish_authentication(&credential)
        .await
        .map_err(|e| e.to_string())
}
```

### 2. Frontend Components

#### Registration Component
Location: `apps/saorsa/src/components/PasskeyRegistration.tsx`

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import { startRegistration } from '@simplewebauthn/browser';

export function PasskeyRegistration() {
    const [username, setUsername] = useState('');
    const [registering, setRegistering] = useState(false);
    
    async function handleRegister() {
        try {
            setRegistering(true);
            
            // Get challenge from backend
            const options = await invoke<CredentialCreationOptions>(
                'start_passkey_registration',
                { username }
            );
            
            // Create credential using WebAuthn
            const credential = await startRegistration(options);
            
            // Complete registration
            const identity = await invoke<Identity>(
                'complete_passkey_registration',
                { credential }
            );
            
            // Store identity locally
            await storeIdentity(identity);
            
            // Navigate to main app
            navigate('/app');
        } catch (error) {
            console.error('Registration failed:', error);
            showError('Failed to register passkey');
        } finally {
            setRegistering(false);
        }
    }
    
    return (
        <div className="passkey-registration">
            <h2>Create Your Identity</h2>
            <input
                type="text"
                placeholder="Choose a username"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
            />
            <button 
                onClick={handleRegister}
                disabled={!username || registering}
            >
                {registering ? 'Creating...' : 'Create Identity'}
            </button>
        </div>
    );
}
```

### 3. DHT Identity Storage

#### Identity Record Structure
```rust
#[derive(Serialize, Deserialize)]
pub struct DhtIdentityRecord {
    pub public_key: Vec<u8>,
    pub passkey_credential_id: Vec<u8>,
    pub three_words: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl IdentityDhtStorage {
    pub async fn store_identity(&self, identity: &NodeIdentity) -> Result<()> {
        let record = DhtIdentityRecord {
            public_key: identity.public_key.to_vec(),
            passkey_credential_id: identity.passkey_id.clone(),
            three_words: identity.three_words.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: identity.metadata.clone(),
        };
        
        // Store with three-word address as key
        let key = format!("identity:{}", identity.three_words);
        self.dht
            .store(&key, &record)
            .await
            .context("Failed to store identity in DHT")?;
        
        // Store reverse lookup
        let reverse_key = format!("cred:{}", hex::encode(&identity.passkey_id));
        self.dht
            .store(&reverse_key, &identity.three_words)
            .await
            .context("Failed to store credential mapping")?;
        
        Ok(())
    }
}
```

### 4. Identity Resolution
```rust
pub async fn resolve_identity_by_three_words(
    &self,
    three_words: &str,
) -> Result<Option<NodeIdentity>> {
    let key = format!("identity:{}", three_words);
    
    match self.dht.get(&key).await? {
        Some(record) => {
            let identity = NodeIdentity::try_from(record)?;
            Ok(Some(identity))
        }
        None => Ok(None),
    }
}
```

### 5. Security Considerations
- Validate all inputs from frontend
- Use secure random for challenges
- Implement rate limiting on registration
- Add audit logging for identity operations
- Store passkey credentials securely

## Testing Requirements
- Unit tests for passkey manager
- Integration tests with mock WebAuthn
- End-to-end tests with real browser
- DHT storage/retrieval tests
- Security audit of auth flow

## Dependencies
- Previous: Task 004 (Identity module safety)
- Previous: Task 003 (DHT module safety)
- External: webauthn-rs crate

## Time Estimate
- Backend implementation: 8 hours
- Frontend components: 6 hours
- DHT integration: 4 hours
- Testing: 4 hours
- Total: 22 hours

## Definition of Done
- [ ] Passkey registration working end-to-end
- [ ] Authentication flow complete
- [ ] DHT storage implemented
- [ ] UI components polished
- [ ] Security review completed