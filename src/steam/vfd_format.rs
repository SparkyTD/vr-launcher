use anyhow::{ensure, Context};
use byteorder::{LittleEndian, ReadBytesExt};
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
use std::ops::Index;
use std::path::PathBuf;
use serde::Serialize;

#[allow(dead_code)]
pub struct AppInfoDatabase {
    pub universe: EUniverse,
    pub entries: Vec<AppInfoEntry>,
}

#[allow(dead_code)]
pub struct AppInfoEntry {
    pub app_id: u32,
    pub info_state: u32,
    pub last_updated: u32,
    pub token: u64,
    pub text_hash: [u8; 20],
    pub change_number: u32,
    pub binary_hash: [u8; 20],
    pub data: HashMap<String, KVValue>,
}

impl AppInfoEntry {
    // Add this method to deserialize the KeyValue data
    pub fn deserialize_kv_data(&mut self, cursor: &mut Cursor<Vec<u8>>, string_pool: &[String]) -> anyhow::Result<()> {
        let mut deserializer = KV1BinaryDeserializer::new(cursor, string_pool);
        self.data = deserializer.read_object()?;

        Ok(())
    }
}


impl AppInfoDatabase {
    pub fn load_from(path: PathBuf) -> anyhow::Result<Self> {
        let mut file = std::fs::File::open(path)?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)?;

        let mut cursor = Cursor::new(buffer);

        ensure!(cursor.read_u32::<LittleEndian>()? == 123094057);

        let universe = EUniverse::try_from(cursor.read_u32::<LittleEndian>()?)?;
        let str_table_offset = cursor.read_u64::<LittleEndian>()?;
        let offset = cursor.position() as usize;
        cursor.seek(SeekFrom::Start(str_table_offset))?;

        let string_count = cursor.read_u32::<LittleEndian>()?;
        let mut string_pool = Vec::with_capacity(string_count as usize);
        let mut buf_reader = BufReader::new(cursor);
        for _ in 0..string_count {
            let mut bytes = Vec::new();
            buf_reader.read_until(0, &mut bytes)?;

            if let Some(0) = bytes.last() {
                bytes.pop();
            }

            string_pool.push(String::from_utf8(bytes)?);
        }

        cursor = buf_reader.into_inner();
        cursor.seek(SeekFrom::Start(offset as u64))?;

        let mut database = AppInfoDatabase {
            universe,
            entries: Vec::new(),
        };

        loop {
            let app_id = cursor.read_u32::<LittleEndian>()?;
            if app_id == 0 {
                break;
            }

            let size_until_data_end = cursor.read_u32::<LittleEndian>()?;
            let data_end = cursor.position() as u32 + size_until_data_end;

            let mut app_entry = AppInfoEntry {
                app_id,
                info_state: cursor.read_u32::<LittleEndian>()?,
                last_updated: cursor.read_u32::<LittleEndian>()?,
                token: cursor.read_u64::<LittleEndian>()?,
                text_hash: {
                    let mut data = vec![0u8; 20];
                    cursor.read_exact(&mut data)?;
                    data.try_into().unwrap()
                },
                change_number: cursor.read_u32::<LittleEndian>()?,
                binary_hash: {
                    let mut data = vec![0u8; 20];
                    cursor.read_exact(&mut data)?;
                    data.try_into().unwrap()
                },
                data: HashMap::new(),
            };

            app_entry.deserialize_kv_data(&mut cursor, &string_pool)?;

            ensure!(cursor.position() == data_end as u64);
            database.entries.push(app_entry);
        }

        Ok(database)
    }

    pub fn app_by_id(&self, app_id: u32) -> Option<&AppInfoEntry> {
        self.entries.iter().find(|e| e.app_id == app_id)
    }
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum EUniverse {
    Invalid = 0,
    Public = 1,
    Beta = 2,
    Internal = 3,
    Dev = 4,
    Max = 5,
}


#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum KVValue {
    String(String),
    Int32(i32),
    UInt64(u64),
    Int64(i64),
    Float32(f32),
    Object(HashMap<String, KVValue>),
    None,
}

#[allow(dead_code)]
impl KVValue {
    pub fn as_object(&self) -> Option<&HashMap<String, KVValue>> {
        match self {
            KVValue::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&String> {
        match self {
            KVValue::String(string) => Some(string),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            KVValue::Int32(value) => Some(*value),
            _ => None,
        }
    }

    pub fn is_string_and<F>(&self, f: F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        self.as_string().is_some_and(|s| f(s.as_str()))
    }

    pub fn is_i32_and<F>(&self, f: F) -> bool
    where
        F: Fn(i32) -> bool,
    {
        self.as_i32().is_some_and(|s| f(s))
    }

    pub fn parse_i32_and<F>(&self, f: F) -> bool
    where
        F: Fn(i32) -> bool,
    {
        match self {
            KVValue::Int32(i) => f(*i),
            KVValue::String(str) => str.parse::<i32>().is_ok_and(|i| f(i)),
            _ => false
        }
    }
    
    pub fn is_none(&self) -> bool {
        match self {
            KVValue::None => true,
            _ => false,
        }
    }
}

impl Index<&str> for KVValue {
    type Output = KVValue;

    fn index(&self, index: &str) -> &Self::Output {
        // obj1.obj2.prop
        let parts = index.split('.').collect::<Vec<&str>>();
        let mut value = self;
        for part in parts {
            match value {
                KVValue::Object(map) => {
                    match map.get(part) {
                        Some(val) => value = val,
                        None => return &KVValue::None
                    }
                }
                _ => return &KVValue::None
            }
        }

        value
    }
}

#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum KV1BinaryNodeType
{
    ChildObject = 0,
    String = 1,
    Int32 = 2,
    Float32 = 3,
    Pointer = 4,
    WideString = 5,
    Color = 6,
    UInt64 = 7,
    End = 8,
    ProbablyBinary = 9,
    Int64 = 10,
    AlternateEnd = 11,
}

struct KV1BinaryDeserializer<'a> {
    cursor: &'a mut Cursor<Vec<u8>>,
    string_pool: &'a [String],
    end_marker: KV1BinaryNodeType,
}

impl<'a> KV1BinaryDeserializer<'a> {
    const BINARY_MAGIC_HEADER: u32 = 0x564B4256;

    fn new(cursor: &'a mut Cursor<Vec<u8>>, string_pool: &'a [String]) -> Self {
        Self {
            cursor,
            string_pool,
            end_marker: KV1BinaryNodeType::End,
        }
    }

    fn read_object(&mut self) -> anyhow::Result<HashMap<String, KVValue>> {
        self.detect_magic_header()?;
        self.read_object_core()
    }

    fn read_object_core(&mut self) -> anyhow::Result<HashMap<String, KVValue>> {
        let mut object = HashMap::new();

        loop {
            let node_type = self.read_next_node_type()?;

            if node_type == self.end_marker {
                break;
            }

            let (key, value) = self.read_value(node_type)?;
            object.insert(key, value);
        }

        Ok(object)
    }

    fn read_key_for_next_value(&mut self) -> anyhow::Result<String> {
        if !self.string_pool.is_empty() {
            let index = self.cursor.read_u32::<LittleEndian>()? as usize;
            ensure!(index < self.string_pool.len(), "String table index out of bounds");
            Ok(self.string_pool[index].clone())
        } else {
            self.read_null_terminated_utf8_string()
        }
    }

    fn read_value(&mut self, node_type: KV1BinaryNodeType) -> anyhow::Result<(String, KVValue)> {
        let name = self.read_key_for_next_value()?;

        let value = match node_type {
            KV1BinaryNodeType::ChildObject => {
                let child_object = self.read_object_core()?;
                KVValue::Object(child_object)
            }

            KV1BinaryNodeType::String => {
                let string_value = self.read_null_terminated_utf8_string()?;
                KVValue::String(string_value)
            }

            KV1BinaryNodeType::WideString => {
                anyhow::bail!("Wide String is not supported");
            }

            KV1BinaryNodeType::Int32 |
            KV1BinaryNodeType::Color |
            KV1BinaryNodeType::Pointer => {
                let int_value = self.cursor.read_i32::<LittleEndian>()?;
                KVValue::Int32(int_value)
            }

            KV1BinaryNodeType::UInt64 => {
                let uint_value = self.cursor.read_u64::<LittleEndian>()?;
                KVValue::UInt64(uint_value)
            }

            KV1BinaryNodeType::Float32 => {
                let float_value = self.cursor.read_f32::<LittleEndian>()?;
                KVValue::Float32(float_value)
            }

            KV1BinaryNodeType::ProbablyBinary => {
                anyhow::bail!("Hit kv type 9 (ProbablyBinary), not supported");
            }

            KV1BinaryNodeType::Int64 => {
                let long_value = self.cursor.read_i64::<LittleEndian>()?;
                KVValue::Int64(long_value)
            }

            _ => {
                anyhow::bail!("Unhandled binary node type: {:?}", node_type);
            }
        };

        Ok((name, value))
    }

    fn read_null_terminated_utf8_string(&mut self) -> anyhow::Result<String> {
        let mut bytes = Vec::new();

        loop {
            let byte = self.cursor.read_u8().context("Failed to read byte for string")?;

            if byte == 0 {
                break;
            }

            bytes.push(byte);
        }

        String::from_utf8(bytes).context("Invalid UTF-8 in string")
    }

    fn detect_magic_header(&mut self) -> anyhow::Result<()> {
        let current_pos = self.cursor.position();
        let remaining = self.cursor.get_ref().len() as u64 - current_pos;

        if remaining < 8 {
            return Ok(());
        }

        let magic = self.cursor.read_u32::<LittleEndian>()?;

        if magic == Self::BINARY_MAGIC_HEADER {
            // Skip CRC32
            self.cursor.seek(SeekFrom::Current(4))?;
            self.end_marker = KV1BinaryNodeType::AlternateEnd;
        } else {
            // Go back as we did not detect the header
            self.cursor.seek(SeekFrom::Current(-4))?;
        }

        Ok(())
    }

    fn read_next_node_type(&mut self) -> anyhow::Result<KV1BinaryNodeType> {
        let type_byte = self.cursor.read_u8()?;
        KV1BinaryNodeType::try_from(type_byte)
            .map_err(|_| anyhow::anyhow!("Invalid node type: {}", type_byte))
    }
}