use crate::utils::*;
use botan_sys::*;

#[derive(Debug)]
/// A raw block cipher interface (ie ECB mode)
///
/// Warning: you almost certainly want an AEAD cipher mode instead
pub struct BlockCipher {
    obj: botan_block_cipher_t,
    block_size: usize,
    min_keylen: usize,
    max_keylen: usize,
    mod_keylen: usize,
}

botan_impl_drop!(BlockCipher, botan_block_cipher_destroy);

impl BlockCipher {
    /// Create a new block cipher instance, failing if the cipher is unknown
    ///
    /// # Examples
    ///
    /// ```
    /// let cipher = botan::BlockCipher::new("AES-128");
    /// assert!(cipher.is_ok());
    /// let no_such_cipher = botan::BlockCipher::new("SuperCipher9000");
    /// assert!(no_such_cipher.is_err());
    /// ```
    pub fn new(name: &str) -> Result<BlockCipher> {
        let obj = botan_init!(botan_block_cipher_init, make_cstr(name)?.as_ptr())?;

        let block_size = {
            let rc = unsafe { botan_block_cipher_block_size(obj) };
            if rc < 0 {
                return Err(Error::from_rc(rc));
            }
            rc as usize
        };

        let (min_keylen, max_keylen, mod_keylen) =
            botan_usize3!(botan_block_cipher_get_keyspec, obj)?;

        Ok(BlockCipher {
            obj,
            block_size,
            min_keylen,
            max_keylen,
            mod_keylen,
        })
    }

    /// Return the block size of the cipher, in bytes
    ///
    /// # Examples
    ///
    /// ```
    /// let cipher = botan::BlockCipher::new("AES-128").unwrap();
    /// assert_eq!(cipher.block_size().unwrap(), 16);
    /// ```
    pub fn block_size(&self) -> Result<usize> {
        Ok(self.block_size)
    }

    /// Return the name of this algorithm which may or may not exactly
    /// match what was provided to new()
    ///
    /// # Examples
    ///
    /// ```
    /// let cipher = botan::BlockCipher::new("AES-128").unwrap();
    /// assert_eq!(cipher.algo_name().unwrap(), "AES-128");
    /// ```
    pub fn algo_name(&self) -> Result<String> {
        call_botan_ffi_returning_string(32, &|out_buf, out_len| unsafe {
            botan_block_cipher_name(self.obj, out_buf as *mut c_char, out_len)
        })
    }

    /// Return information about the key lengths supported by this object
    pub fn key_spec(&self) -> Result<KeySpec> {
        KeySpec::new(self.min_keylen, self.max_keylen, self.mod_keylen)
    }

    /// Set the key for the cipher.
    ///
    /// # Errors
    ///
    /// Fails if the key is not a valid length for the cipher
    ///
    /// # Examples
    ///
    /// ```
    /// let mut cipher = botan::BlockCipher::new("AES-128").unwrap();
    /// assert!(cipher.set_key(&vec![0; 32]).is_err());
    /// assert!(cipher.set_key(&vec![0; 16]).is_ok());
    /// ```
    pub fn set_key(&mut self, key: &[u8]) -> Result<()> {
        botan_call!(
            botan_block_cipher_set_key,
            self.obj,
            key.as_ptr(),
            key.len()
        )
    }

    /// Encrypt some blocks of data
    ///
    /// # Errors
    ///
    /// Fails if the input is not a multiple of the block size, or if the
    /// key was not set on the object.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut cipher = botan::BlockCipher::new("AES-128").unwrap();
    /// // Key is not set
    /// assert!(cipher.encrypt_blocks(&vec![0; 16]).is_err());
    /// assert!(cipher.set_key(&vec![0; 16]).is_ok());
    /// // Not a multiple of block size
    /// assert!(cipher.encrypt_blocks(&vec![0; 17]).is_err());
    /// // Key is set and multiple of block size - ok
    /// assert!(cipher.encrypt_blocks(&vec![0; 16]).is_ok());
    /// ```
    pub fn encrypt_blocks(&self, input: &[u8]) -> Result<Vec<u8>> {
        let mut ivec = input.to_vec();
        self.encrypt_in_place(&mut ivec)?;
        Ok(ivec)
    }

    /// Encrypt in place
    ///
    /// # Errors
    ///
    /// Fails if the input is not a multiple of the block size, or if the
    /// key was not set on the object.
    pub fn encrypt_in_place(&self, buf: &mut [u8]) -> Result<()> {
        if buf.len() % self.block_size != 0 {
            return Err(Error::with_message(
                ErrorType::InvalidInput,
                "Invalid input size".to_string(),
            ));
        }

        let blocks = buf.len() / self.block_size;

        botan_call!(
            botan_block_cipher_encrypt_blocks,
            self.obj,
            buf.as_ptr(),
            buf.as_mut_ptr(),
            blocks
        )
    }

    /// Decrypt some blocks of data
    ///
    /// # Errors
    ///
    /// Fails if the input is not a multiple of the block size, or if the
    /// key was not set on the object.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut cipher = botan::BlockCipher::new("AES-128").unwrap();
    /// // Key is not set
    /// assert!(cipher.decrypt_blocks(&vec![0; 16]).is_err());
    /// assert!(cipher.set_key(&vec![0; 16]).is_ok());
    /// // Not a multiple of block size
    /// assert!(cipher.decrypt_blocks(&vec![0; 17]).is_err());
    /// // Key is set and multiple of block size - ok
    /// assert!(cipher.decrypt_blocks(&vec![0; 16]).is_ok());
    /// ```
    pub fn decrypt_blocks(&self, input: &[u8]) -> Result<Vec<u8>> {
        let mut ivec = input.to_vec();
        self.decrypt_in_place(&mut ivec)?;
        Ok(ivec)
    }

    /// Decrypt in place
    ///
    /// # Errors
    ///
    /// Fails if the input is not a multiple of the block size, or if the
    /// key was not set on the object.
    pub fn decrypt_in_place(&self, buf: &mut [u8]) -> Result<()> {
        if buf.len() % self.block_size != 0 {
            return Err(Error::with_message(
                ErrorType::InvalidInput,
                "Invalid input size".to_string(),
            ));
        }

        let blocks = buf.len() / self.block_size;

        botan_call!(
            botan_block_cipher_decrypt_blocks,
            self.obj,
            buf.as_ptr(),
            buf.as_mut_ptr(),
            blocks
        )
    }

    /// Clear the key set on the cipher from memory. After this, the
    /// object is un-keyed and must be re-keyed before use.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut cipher = botan::BlockCipher::new("AES-128").unwrap();
    /// assert!(cipher.set_key(&vec![0; 16]).is_ok());
    /// assert!(cipher.encrypt_blocks(&vec![0; 16]).is_ok());
    /// assert!(cipher.clear().is_ok());
    /// assert!(cipher.encrypt_blocks(&vec![0; 16]).is_err());
    /// ```
    pub fn clear(&mut self) -> Result<()> {
        botan_call!(botan_block_cipher_clear, self.obj)
    }
}
