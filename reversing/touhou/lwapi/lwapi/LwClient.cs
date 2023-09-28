using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Threading.Tasks;

namespace lwapi
{
    public class LwClient
    {
        private HttpClient http;
        private string encryption_key = "TpzWfhPBWtcNbUScHM2hM6bpw58Tm3Ji";
        private string api_server = "https://g-api.touhoulostword.com/app/";
        private string? device_id;
        private string? session_id;

        public string EncryptionKey => encryption_key;
        public string ApiServer => api_server;
        public string? DeviceId => device_id;
        public string? SessionId => session_id;

        public LwClient()
        {
            http = new HttpClient();
            http.DefaultRequestHeaders.Add("User-Agent", "UnityPlayer/2021.3.11f1 (UnityWebRequest/1.0, libcurl/7.80.0-DEV)");
            http.DefaultRequestHeaders.Add("X-Unity-Version", "2021.3.11f1");

            var initRes = Send<GameInitRequest, GameInitResponse>(new GameInitRequest());
            if (initRes != null)
                api_server = Encoding.UTF8.GetString(Convert.FromBase64String(initRes.api_server));
            else
                throw new Exception("Init request failed");
        }

        public TRes? Send<TReq, TRes>(TReq req)
            where TReq : RequestPacket
            where TRes : ResponsePacket
        {
            var packet = new SendPacket()
            {
                device_id = device_id,
                session_id = session_id,
                resource_version = 0,
                app_version = 43,
                body = httpEncrypt(JsonConvert.SerializeObject(req))
            };
            var httpReq = new HttpRequestMessage(HttpMethod.Post, $"{api_server}{req.GetAPIName()}")
            {
                Content = new StringContent(JsonConvert.SerializeObject(packet), Encoding.UTF8, "application/json"),
            };

            using var reader = new StreamReader(http.Send(httpReq).Content.ReadAsStream());
            var str = reader.ReadToEnd();
            var resPacket = JsonConvert.DeserializeObject<RecvPacket>(str);
            if (resPacket?.body != null)
            {
                var dec = httpDecrypt(resPacket.body);
                var body = req is GameInitRequest ? Encoding.UTF8.GetString(dec) : zlibInflate(dec);
                var res = JsonConvert.DeserializeObject<TRes>(body);
                if (res != null)
                {
                    if (res.IsEncryptionKeyUpdate())
                        encryption_key = res.encryption_key;
                    session_id = res.session_id;
                    return res;
                }
            }
            return null;
        }

        private static AesManaged getAes(string key)
        {
            return new AesManaged()
            {
                BlockSize = 128,
                KeySize = key.Length * 8,
                Padding = PaddingMode.PKCS7,
                Mode = CipherMode.CBC,
                Key = Encoding.UTF8.GetBytes(key),
                IV = new byte[16],
            };
        }

        private string httpEncrypt(string input)
        {
            var aes = getAes(encryption_key);
            var enc = Encoding.UTF8.GetBytes(input);
            return Convert.ToBase64String(aes.CreateEncryptor().TransformFinalBlock(enc, 0, enc.Length));
        }

        private byte[] httpDecrypt(string input)
        {
            var aes = getAes(encryption_key);
            var enc = Convert.FromBase64String(input);
            return aes.CreateDecryptor().TransformFinalBlock(enc, 0, enc.Length);
        }

        private string zlibInflate(byte[] input)
        {
            using var mem = new MemoryStream(input);
            using var zlib = new ZLibStream(mem, CompressionMode.Decompress);
            using var reader = new StreamReader(zlib);
            return reader.ReadToEnd();
        }
    }
}
