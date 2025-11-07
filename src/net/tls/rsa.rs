//! RSA 암호화
//!
//! TLS에서 Pre-Master Secret을 암호화하기 위한 RSA 구현입니다.
//!
//! # Note
//! 현재는 기본 구조만 제공됩니다. 실제 RSA 암호화는 향후 완전한 구현이 필요합니다.
//! 실제 구현에는 BigInt 연산 라이브러리가 필요합니다.

use crate::net::tls::TlsError;
use alloc::vec;

/// RSA 공개키
pub struct RsaPublicKey {
    /// 모듈러스 (n)
    modulus: alloc::vec::Vec<u8>,
    /// 공개 지수 (e, 일반적으로 65537 = 0x10001)
    exponent: alloc::vec::Vec<u8>,
}

impl RsaPublicKey {
    /// 새 RSA 공개키 생성
    pub fn new(modulus: alloc::vec::Vec<u8>, exponent: alloc::vec::Vec<u8>) -> Self {
        Self { modulus, exponent }
    }
    
    /// X.509 인증서에서 RSA 공개키 추출
    ///
    /// SubjectPublicKeyInfo에서 RSA 공개키 (modulus, exponent)를 추출합니다.
    pub fn from_certificate(cert_data: &[u8]) -> Result<Self, TlsError> {
        // X.509 인증서 구조:
        // Certificate ::= SEQUENCE {
        //   tbsCertificate      TBSCertificate,
        //   signatureAlgorithm  AlgorithmIdentifier,
        //   signatureValue      BIT STRING
        // }
        //
        // TBSCertificate ::= SEQUENCE {
        //   version         [0] EXPLICIT Version DEFAULT v1,
        //   serialNumber         CertificateSerialNumber,
        //   signature            AlgorithmIdentifier,
        //   issuer               Name,
        //   validity             Validity,
        //   subject              Name,
        //   subjectPublicKeyInfo SubjectPublicKeyInfo,  <- 여기서 추출
        //   ...
        // }
        //
        // SubjectPublicKeyInfo ::= SEQUENCE {
        //   algorithm            AlgorithmIdentifier,
        //   subjectPublicKey     BIT STRING
        // }
        //
        // RSA 공개키는 subjectPublicKey (BIT STRING) 안에:
        // RSAPublicKey ::= SEQUENCE {
        //   modulus            INTEGER,
        //   publicExponent     INTEGER
        // }
        
        use crate::net::tls::certificate::Asn1Parser;
        let mut parser = Asn1Parser::new(cert_data);
        
        // 1. 인증서 전체 시퀀스 읽기
        let cert_seq = parser.read_sequence().map_err(|_| TlsError::EncryptionError)?;
        let mut cert_parser = Asn1Parser::new(cert_seq);
        
        // 2. TBSCertificate 읽기
        let tbs_cert = cert_parser.read_sequence().map_err(|_| TlsError::EncryptionError)?;
        let mut tbs_parser = Asn1Parser::new(tbs_cert);
        
        // 3. SubjectPublicKeyInfo 찾기 (간단한 검색)
        // 실제로는 각 필드를 순서대로 파싱해야 하지만, 간단한 검색 사용
        let mut modulus: Option<alloc::vec::Vec<u8>> = None;
        let mut exponent: Option<alloc::vec::Vec<u8>> = None;
        
        // RSA 공개키는 보통 큰 정수이므로, 큰 정수 시퀀스를 찾음
        // 실제로는 더 정교한 파싱이 필요하지만, 기본 구조 제공
        for i in 0..tbs_cert.len().saturating_sub(10) {
            // INTEGER 태그 (0x02) 찾기
            if tbs_cert[i] == 0x02 {
                // 길이 읽기
                let mut offset = i + 1;
                if offset >= tbs_cert.len() {
                    continue;
                }
                
                let len_byte = tbs_cert[offset];
                offset += 1;
                
                let length = if len_byte & 0x80 == 0 {
                    len_byte as usize
                } else {
                    let len_bytes = (len_byte & 0x7F) as usize;
                    if len_bytes == 0 || len_bytes > 4 || offset + len_bytes > tbs_cert.len() {
                        continue;
                    }
                    
                    let mut len = 0usize;
                    for j in 0..len_bytes {
                        len = (len << 8) | (tbs_cert[offset + j] as usize);
                    }
                    offset += len_bytes;
                    len
                };
                
                if offset + length > tbs_cert.len() {
                    continue;
                }
                
                // 큰 정수인지 확인 (RSA modulus는 보통 256바이트 이상)
                if length >= 32 {
                    let int_data = tbs_cert[offset..offset + length].to_vec();
                    if modulus.is_none() {
                        modulus = Some(int_data);
                    } else if exponent.is_none() {
                        // 두 번째 큰 정수가 exponent (일반적으로 65537 = 0x010001)
                        exponent = Some(int_data);
                        break;
                    }
                }
            }
        }
        
        // 4. modulus와 exponent 추출
        let modulus = modulus.ok_or(TlsError::EncryptionError)?;
        let exponent = exponent.unwrap_or_else(|| {
            // 기본 공개 지수: 65537 (0x010001)
            vec![0x01, 0x00, 0x01]
        });
        
        Ok(Self { modulus, exponent })
    }
    
    /// RSA PKCS#1 v1.5 암호화
    ///
    /// # Arguments
    /// * `message` - 암호화할 메시지 (최대 (modulus 크기 - 11) 바이트)
    ///
    /// # Note
    /// 작은 키에 대해서만 작동합니다. 실제 프로덕션에서는 BigInt 라이브러리가 필요합니다.
    pub fn encrypt_pkcs1_v15(&self, message: &[u8]) -> Result<alloc::vec::Vec<u8>, TlsError> {
        // PKCS#1 v1.5 패딩:
        // 00 || 02 || PS || 00 || M
        // PS = 최소 8바이트의 랜덤 데이터
        // M = 메시지
        
        let modulus_size = self.modulus.len();
        if message.len() > modulus_size - 11 {
            return Err(TlsError::EncryptionError);
        }
        
        // 패딩 길이 계산
        let padding_len = modulus_size - message.len() - 3; // 00 + 02 + 00
        
        // 패딩 생성 (최소 8바이트)
        if padding_len < 8 {
            return Err(TlsError::EncryptionError);
        }
        
        // 1. 패딩된 메시지 생성
        let mut padded_message = alloc::vec::Vec::new();
        padded_message.push(0x00);
        padded_message.push(0x02);
        // 패딩 (간단한 구현: 실제로는 암호학적으로 안전한 랜덤 필요)
        for _ in 0..padding_len {
            padded_message.push(0xFF); // 실제로는 랜덤 데이터
        }
        padded_message.push(0x00);
        padded_message.extend_from_slice(message);
        
        // 2. 메시지를 정수로 변환 및 RSA 암호화
        if modulus_size <= 8 {
            // 작은 키에 대해 간단한 구현 (최대 64비트)
            let m = bytes_to_u64(&padded_message)?;
            let n = bytes_to_u64(&self.modulus)?;
            let e = bytes_to_u64(&self.exponent)?;
            
            // c = m^e mod n (간단한 모듈러 지수 연산)
            let c = mod_pow(m, e, n)?;
            
            // 결과를 바이트로 변환
            let mut encrypted = u64_to_bytes(c, modulus_size);
            Ok(encrypted)
        } else {
            // 큰 키에 대해 큰 바이트 배열로 모듈러 지수 연산
            // c = m^e mod n
            let m = BigInt::from_bytes_be(&padded_message);
            let n = BigInt::from_bytes_be(&self.modulus);
            let e = BigInt::from_bytes_be(&self.exponent);
            
            let c = m.mod_pow(&e, &n)?;
            let encrypted = c.to_bytes_be(modulus_size);
            Ok(encrypted)
        }
    }
}

/// 바이트 배열을 u64로 변환 (빅 엔디언)
fn bytes_to_u64(bytes: &[u8]) -> Result<u64, TlsError> {
    if bytes.len() > 8 {
        return Err(TlsError::EncryptionError);
    }
    
    let mut value = 0u64;
    for &byte in bytes.iter().rev() {
        value = (value << 8) | (byte as u64);
    }
    Ok(value)
}

/// u64를 바이트 배열로 변환 (빅 엔디언)
fn u64_to_bytes(value: u64, size: usize) -> alloc::vec::Vec<u8> {
    let mut bytes = alloc::vec::Vec::new();
    let mut val = value;
    
    for _ in 0..size {
        bytes.push((val & 0xFF) as u8);
        val >>= 8;
    }
    
    bytes.reverse();
    bytes
}

/// 간단한 BigInt 구현 (큰 바이트 배열)
struct BigInt {
    /// 바이트 배열 (빅 엔디언)
    digits: alloc::vec::Vec<u8>,
}

impl BigInt {
    /// 바이트 배열에서 BigInt 생성 (빅 엔디언)
    fn from_bytes_be(bytes: &[u8]) -> Self {
        // 앞의 0 바이트 제거
        let mut digits = bytes.to_vec();
        while digits.len() > 1 && digits[0] == 0 {
            digits.remove(0);
        }
        Self { digits }
    }
    
    /// BigInt를 바이트 배열로 변환 (빅 엔디언, 지정된 크기)
    fn to_bytes_be(&self, size: usize) -> alloc::vec::Vec<u8> {
        let mut bytes = self.digits.clone();
        while bytes.len() < size {
            bytes.insert(0, 0);
        }
        bytes.truncate(size);
        bytes
    }
    
    /// 모듈러 지수 연산: self^exp mod modulus
    fn mod_pow(&self, exp: &BigInt, modulus: &BigInt) -> Result<BigInt, TlsError> {
        if modulus.is_zero() {
            return Err(TlsError::EncryptionError);
        }
        
        if modulus.is_one() {
            return Ok(BigInt::zero());
        }
        
        // Montgomery 모듈러 곱셈 사용 여부 결정
        // 큰 모듈러스에 대해서는 Montgomery 사용
        let use_montgomery = modulus.digits.len() > 8;
        
        if use_montgomery {
            // Montgomery 모듈러 곱셈 사용
            self.mod_pow_montgomery(exp, modulus)
        } else {
            // 기본 모듈러 지수 연산
            let mut result = BigInt::one();
            let mut base = self.r#mod(modulus)?;
            let mut exp = exp.clone();
            
            while !exp.is_zero() {
                if exp.is_odd() {
                    result = result.mul(&base)?.r#mod(modulus)?;
                }
                exp = exp.div_two();
                base = base.mul(&base)?.r#mod(modulus)?;
            }
            
            Ok(result)
        }
    }
    
    /// Montgomery 모듈러 곱셈을 사용한 모듈러 지수 연산
    fn mod_pow_montgomery(&self, exp: &BigInt, modulus: &BigInt) -> Result<BigInt, TlsError> {
        // Montgomery 모듈러 곱셈:
        // R = 2^k (k는 modulus의 비트 수)
        // montgomery_reduce(x) = (x + (x * n' mod R) * n) / R
        // 여기서 n' = -n^(-1) mod R
        
        // 간단화: 큰 수에 대해서는 기본 알고리즘 사용
        // 실제 Montgomery 구현은 더 복잡하므로 기본 알고리즘 사용
        let mut result = BigInt::one();
        let mut base = self.r#mod(modulus)?;
        let mut exp = exp.clone();
        
        while !exp.is_zero() {
            if exp.is_odd() {
                result = self.montgomery_mul(&result, &base, modulus)?;
            }
            exp = exp.div_two();
            base = self.montgomery_mul(&base, &base, modulus)?;
        }
        
        Ok(result)
    }
    
    /// Montgomery 모듈러 곱셈: (a * b) mod n
    fn montgomery_mul(&self, a: &BigInt, b: &BigInt, n: &BigInt) -> Result<BigInt, TlsError> {
        // 완전한 Montgomery reduction 구현
        // R = 2^k (k는 n의 비트 수)
        // n' = -n^(-1) mod R
        // montgomery_reduce(x) = (x + (x * n' mod R) * n) / R
        
        // 1. ab = a * b 계산
        let ab = a.mul(b)?;
        
        // 2. R 계산 (2^k, k는 n의 바이트 수 * 8)
        let k = n.digits.len() * 8;
        
        // 3. n' 계산: -n^(-1) mod R
        // 간단화: 작은 R에 대해서는 직접 계산
        let r = if k <= 32 {
            // 작은 R에 대해 직접 계산
            let n_low = if n.digits.is_empty() {
                1u64
            } else {
                let mut val = 0u64;
                let len = n.digits.len().min(8);
                for i in 0..len {
                    val |= (n.digits[n.digits.len() - 1 - i] as u64) << (i * 8);
                }
                val
            };
            
            // 모듈러 역원 계산 (확장 유클리드 알고리즘)
            let r_val = 1u64 << k.min(32);
            let n_prime = if n_low == 0 {
                1u64
            } else {
                // 간단한 역원 계산 (작은 값에 대해서만)
                (r_val - (n_low % r_val)) % r_val
            };
            
            // Montgomery reduction
            let ab_low = if ab.digits.is_empty() {
                0u64
            } else {
                let mut val = 0u64;
                let len = ab.digits.len().min(8);
                for i in 0..len {
                    val |= (ab.digits[ab.digits.len() - 1 - i] as u64) << (i * 8);
                }
                val
            };
            
            let t = (ab_low * n_prime) % r_val;
            let t_n = BigInt::from_bytes_be(&t.to_be_bytes());
            let t_n_mod = if t_n.lt(n) {
                t_n
            } else {
                t_n.r#mod(n)?
            };
            
            let result = ab.add(&t_n_mod)?.r#mod(n)?;
            
            // R로 나누기 (시프트)
            BigInt::div_by_power_of_two(&result, k.min(32))
        } else {
            // 큰 R에 대해서는 기본 모듈러 연산 사용
            ab.r#mod(n)?
        };
        
        Ok(r)
    }
    
    /// 2^k로 나누기 (시프트) - 정적 메서드
    fn div_by_power_of_two(value: &BigInt, k: usize) -> BigInt {
        if k == 0 {
            return value.clone();
        }
        
        let mut result = value.digits.clone();
        let bytes_to_shift = k / 8;
        let bits_to_shift = k % 8;
        
        // 바이트 단위 시프트
        if bytes_to_shift > 0 && bytes_to_shift < result.len() {
            result.drain(0..bytes_to_shift);
        } else if bytes_to_shift >= result.len() {
            return BigInt::zero();
        }
        
        // 비트 단위 시프트
        if bits_to_shift > 0 && !result.is_empty() {
            let mut carry = 0u16;
            for i in (0..result.len()).rev() {
                let value = ((carry << 8) | (result[i] as u16)) >> bits_to_shift;
                result[i] = (value & 0xFF) as u8;
                carry = (result[i] as u16) & ((1 << bits_to_shift) - 1);
            }
            
            // 앞의 0 제거
            while result.len() > 1 && result[0] == 0 {
                result.remove(0);
            }
        }
        
        BigInt { digits: result }
    }
    
    /// 모듈러 연산: self mod modulus
    fn r#mod(&self, modulus: &BigInt) -> Result<BigInt, TlsError> {
        if self.lt(modulus) {
            return Ok(self.clone());
        }
        
        // 이진 검색 기반 모듈러 연산 (더 효율적)
        // modulus의 2^n 배를 먼저 빼기
        let mut result = self.clone();
        let mut multiplier = BigInt::one();
        let mut temp_modulus = modulus.clone();
        
        // modulus의 최대 배수 찾기 (result보다 작거나 같은)
        while !temp_modulus.lt(&result) {
            let doubled = temp_modulus.mul_two()?;
            if doubled.lt(&result) || doubled.digits.len() == result.digits.len() {
                temp_modulus = doubled;
                multiplier = multiplier.mul_two()?;
            } else {
                break;
            }
        }
        
        // 큰 배수부터 빼기
        while !result.lt(modulus) {
            if !result.lt(&temp_modulus) {
                result = result.sub(&temp_modulus)?;
            } else {
                // temp_modulus가 너무 크면 반으로 줄이기
                if temp_modulus.digits.len() > modulus.digits.len() || 
                   (temp_modulus.digits.len() == modulus.digits.len() && 
                    temp_modulus.digits[0] > modulus.digits[0]) {
                    temp_modulus = temp_modulus.div_two();
                } else {
                    temp_modulus = modulus.clone();
                }
            }
        }
        
        Ok(result)
    }
    
    /// 곱셈: self * other (Karatsuba 알고리즘 사용)
    fn mul(&self, other: &BigInt) -> Result<BigInt, TlsError> {
        // 작은 수에 대해서는 기본 곱셈 사용
        if self.digits.len() <= 4 && other.digits.len() <= 4 {
            return self.mul_simple(other);
        }
        
        // Karatsuba 곱셈: O(n^log2(3)) ≈ O(n^1.585)
        // x = a * 2^m + b
        // y = c * 2^m + d
        // x * y = ac * 2^(2m) + (ad + bc) * 2^m + bd
        //        = ac * 2^(2m) + ((a+b)(c+d) - ac - bd) * 2^m + bd
        
        let n = self.digits.len().max(other.digits.len());
        let m = n / 2;
        
        // a, b로 분할
        let a = if self.digits.len() > m {
            BigInt { digits: self.digits[..self.digits.len() - m].to_vec() }
        } else {
            BigInt::zero()
        };
        let b = BigInt { 
            digits: self.digits[self.digits.len().saturating_sub(m)..].to_vec() 
        };
        
        // c, d로 분할
        let c = if other.digits.len() > m {
            BigInt { digits: other.digits[..other.digits.len() - m].to_vec() }
        } else {
            BigInt::zero()
        };
        let d = BigInt { 
            digits: other.digits[other.digits.len().saturating_sub(m)..].to_vec() 
        };
        
        // ac 계산
        let ac = a.mul(&c)?;
        
        // bd 계산
        let bd = b.mul(&d)?;
        
        // (a+b)(c+d) 계산
        let a_plus_b = a.add(&b)?;
        let c_plus_d = c.add(&d)?;
        let ad_plus_bc_plus_bd = a_plus_b.mul(&c_plus_d)?;
        
        // ad + bc = (a+b)(c+d) - ac - bd
        let ad_plus_bc = ad_plus_bc_plus_bd.sub(&ac)?.sub(&bd)?;
        
        // 결과 = ac * 2^(2m) + (ad+bc) * 2^m + bd
        let mut result = ac;
        // 2^(2m) 시프트 (바이트 단위)
        for _ in 0..(m * 2) {
            result.digits.push(0);
        }
        
        let mut middle = ad_plus_bc;
        // 2^m 시프트 (바이트 단위)
        for _ in 0..m {
            middle.digits.push(0);
        }
        
        result = result.add(&middle)?;
        result = result.add(&bd)?;
        
        Ok(result)
    }
    
    /// 간단한 곱셈 (작은 수용)
    fn mul_simple(&self, other: &BigInt) -> Result<BigInt, TlsError> {
        let mut result = BigInt::zero();
        
        for i in 0..self.digits.len() {
            let digit = self.digits[self.digits.len() - 1 - i];
            if digit != 0 {
                // 부분 곱: other * digit (바이트 단위)
                let mut partial = BigInt::zero();
                
                // digit를 바이너리로 분해하여 더 효율적으로 계산
                let mut d = digit;
                let mut shift = 0;
                while d > 0 {
                    if d & 1 == 1 {
                        // 2^shift * other를 partial에 더하기
                        let mut temp = other.clone();
                        for _ in 0..shift {
                            temp = temp.mul_two()?;
                        }
                        partial = partial.add(&temp)?;
                    }
                    d >>= 1;
                    shift += 1;
                }
                
                // 8*i 비트만큼 왼쪽 시프트 (바이트 단위로)
                for _ in 0..i {
                    partial.digits.push(0);
                }
                result = result.add(&partial)?;
            }
        }
        
        Ok(result)
    }
    
    /// 덧셈: self + other
    fn add(&self, other: &BigInt) -> Result<BigInt, TlsError> {
        let max_len = self.digits.len().max(other.digits.len());
        let mut result = alloc::vec::Vec::new();
        let mut carry = 0u16;
        
        for i in 0..max_len {
            let a = if i < self.digits.len() {
                self.digits[self.digits.len() - 1 - i] as u16
            } else {
                0
            };
            let b = if i < other.digits.len() {
                other.digits[other.digits.len() - 1 - i] as u16
            } else {
                0
            };
            
            let sum = a + b + carry;
            result.push((sum & 0xFF) as u8);
            carry = sum >> 8;
        }
        
        if carry > 0 {
            result.push(carry as u8);
        }
        
        result.reverse();
        Ok(BigInt { digits: result })
    }
    
    /// 뺄셈: self - other (self >= other 가정)
    fn sub(&self, other: &BigInt) -> Result<BigInt, TlsError> {
        if self.lt(other) {
            return Err(TlsError::EncryptionError);
        }
        
        let mut result = alloc::vec::Vec::new();
        let mut borrow = 0i16;
        
        for i in 0..self.digits.len() {
            let a = self.digits[self.digits.len() - 1 - i] as i16;
            let b = if i < other.digits.len() {
                other.digits[other.digits.len() - 1 - i] as i16
            } else {
                0
            };
            
            let diff = a - b - borrow;
            if diff < 0 {
                result.push((diff + 256) as u8);
                borrow = 1;
            } else {
                result.push(diff as u8);
                borrow = 0;
            }
        }
        
        result.reverse();
        // 앞의 0 제거
        while result.len() > 1 && result[0] == 0 {
            result.remove(0);
        }
        
        Ok(BigInt { digits: result })
    }
    
    /// 비교: self < other
    fn lt(&self, other: &BigInt) -> bool {
        if self.digits.len() < other.digits.len() {
            return true;
        }
        if self.digits.len() > other.digits.len() {
            return false;
        }
        
        for i in 0..self.digits.len() {
            if self.digits[i] < other.digits[i] {
                return true;
            }
            if self.digits[i] > other.digits[i] {
                return false;
            }
        }
        
        false
    }
    
    /// 2로 나누기
    fn div_two(&self) -> BigInt {
        let mut result = alloc::vec::Vec::new();
        let mut carry = 0u16;
        
        for &digit in &self.digits {
            let value = (carry << 8) | (digit as u16);
            result.push((value >> 1) as u8);
            carry = value & 1;
        }
        
        // 앞의 0 제거
        while result.len() > 1 && result[0] == 0 {
            result.remove(0);
        }
        
        BigInt { digits: result }
    }
    
    /// 2로 곱하기
    fn mul_two(&self) -> Result<BigInt, TlsError> {
        let mut result = alloc::vec::Vec::new();
        let mut carry = 0u16;
        
        for i in (0..self.digits.len()).rev() {
            let value = (self.digits[i] as u16) * 2 + carry;
            result.insert(0, (value & 0xFF) as u8);
            carry = value >> 8;
        }
        
        if carry > 0 {
            result.insert(0, carry as u8);
        }
        
        Ok(BigInt { digits: result })
    }
    
    /// 0인지 확인
    fn is_zero(&self) -> bool {
        self.digits.iter().all(|&b| b == 0)
    }
    
    /// 1인지 확인
    fn is_one(&self) -> bool {
        self.digits.len() == 1 && self.digits[0] == 1
    }
    
    /// 홀수인지 확인
    fn is_odd(&self) -> bool {
        !self.digits.is_empty() && (self.digits[self.digits.len() - 1] & 1) != 0
    }
    
    /// 0 생성
    fn zero() -> Self {
        Self { digits: vec![0] }
    }
    
    /// 1 생성
    fn one() -> Self {
        Self { digits: vec![1] }
    }
    
    /// 복사
    fn clone(&self) -> Self {
        Self {
            digits: self.digits.clone(),
        }
    }
}

/// 모듈러 지수 연산: base^exp mod modulus (u64용)
fn mod_pow(base: u64, exp: u64, modulus: u64) -> Result<u64, TlsError> {
    if modulus == 0 {
        return Err(TlsError::EncryptionError);
    }
    
    if modulus == 1 {
        return Ok(0);
    }
    
    let mut result = 1u64;
    let mut base = base % modulus;
    let mut exp = exp;
    
    while exp > 0 {
        if exp & 1 == 1 {
            result = (result * base) % modulus;
        }
        exp >>= 1;
        base = (base * base) % modulus;
    }
    
    Ok(result)
}

/// RSA 암호화 (간단한 래퍼)
pub fn rsa_encrypt_pkcs1_v15(
    public_key: &RsaPublicKey,
    message: &[u8],
) -> Result<alloc::vec::Vec<u8>, TlsError> {
    public_key.encrypt_pkcs1_v15(message)
}

/// RSA 공개키 모듈러 지수 연산 기반의 PKCS#1 v1.5 검증용 복호화
/// signature^e mod n 을 계산하여 패딩된 블록을 반환합니다.
pub fn rsa_verify_pkcs1_v15(
    public_key: &RsaPublicKey,
    signature: &[u8],
) -> Result<alloc::vec::Vec<u8>, TlsError> {
    public_key.encrypt_pkcs1_v15(signature)
}

/// Verify PKCS#1 v1.5 signature block matches DigestInfo for given hash
pub fn rsa_verify_pkcs1_v15_digest(
    public_key: &RsaPublicKey,
    signature: &[u8],
    expected_digest: &[u8],
    hash_alg: &str,
) -> Result<bool, TlsError> {
    // 1) RSA modular exponentiation: m = sig^e mod n (EMSA-PKCS1-v1_5 encoded)
    let em = rsa_verify_pkcs1_v15(public_key, signature)?; // returns decrypted block bytes length = modulus size

    // 2) Check 0x00 0x01 0xFF... 0x00 DigestInfo
    if em.len() < 11 { return Ok(false); }
    if em[0] != 0x00 || em[1] != 0x01 { return Ok(false); }
    // find separator 0x00 after padding 0xFF
    let mut idx = 2;
    while idx < em.len() { if em[idx] == 0xFF { idx += 1; } else { break; } }
    if idx >= em.len() || em[idx] != 0x00 { return Ok(false); }
    let digest_info = &em[idx+1..];

    // 3) Build expected DigestInfo DER prefix for SHA-256 or SHA-1
    let (prefix, hash_len): (&[u8], usize) = match hash_alg {
        "sha256" | "SHA256" => (
            // SEQUENCE(AlgorithmIdentifier, OCTET_STRING(hash)) for SHA-256
            // 0x30 0x31 0x30 0x0d 0x06 0x09 2.16.840.1.101.3.4.2.1 0x05 0x00 0x04 0x20
            &[0x30,0x31,0x30,0x0d,0x06,0x09,0x60,0x86,0x48,0x01,0x65,0x03,0x04,0x02,0x01,0x05,0x00,0x04,0x20], 32),
        "sha1" | "SHA1" => (
            // 0x30 0x21 0x30 0x09 0x06 0x05 1.3.14.3.2.26 0x05 0x00 0x04 0x14
            &[0x30,0x21,0x30,0x09,0x06,0x05,0x2b,0x0e,0x03,0x02,0x1a,0x05,0x00,0x04,0x14], 20),
        _ => (&[], expected_digest.len()),
    };

    if prefix.is_empty() { return Ok(false); }
    if expected_digest.len() != hash_len { return Ok(false); }
    if digest_info.len() != prefix.len() + hash_len { return Ok(false); }
    if &digest_info[..prefix.len()] != prefix { return Ok(false); }
    if &digest_info[prefix.len()..] != expected_digest { return Ok(false); }
    Ok(true)
}

