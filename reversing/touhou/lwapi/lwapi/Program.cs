using System.Security.Cryptography;
using System.Text;
using System;
using System.Net.Http;
using Newtonsoft.Json;
using static System.Net.WebRequestMethods;
using System.IO.Compression;
using System.Xml.Linq;

#nullable disable

namespace lwapi
{
    internal class Program
    {
        public enum DLKey
        {
            AssetBundleManifest = 0,
            Initial = 1,
            Expansion = 2,
        }

        public enum DLType
        {
            AssetBudnle = 0, // sic
            Master = 1,
            Resource = 2,
        }

        class AssetLoaderManifestAssetInfo
        {
            public DLKey Key;
            public DLType Type;
            public string Name;
            public string Hash;
            public uint Crc;
            public long Size;
            public List<string> AssetPaths;
            public List<string> DownloadBundle;
            public string DevelopParam;
        }

        class AssetLoaderManifestResourceInfo
        {
            public long EndAt;
            public string Name;
        }

        class AssetLoaderManifestMasterInfo
        {
            public long EndAt;
            public int Id;
        }

        class AssetLoaderManifest
        {
            public List<AssetLoaderManifestAssetInfo> AssetInfos;
            public List<AssetLoaderManifestResourceInfo> ResourceInfos;
            public List<AssetLoaderManifestMasterInfo> Unit;
            public List<AssetLoaderManifestMasterInfo> Picture;
        }

        static AssetLoaderManifest GetManifest(HttpClient http, string name, string remotePath)
        {
            var httpReq = new HttpRequestMessage(HttpMethod.Get, $"{remotePath}7f5cb74af5d7f4b82200738fdbdc5a45");
            using var stream = new GZipStream(http.Send(httpReq).Content.ReadAsStream(), CompressionMode.Decompress);
            using var reader = new StreamReader(stream);
            var ret = JsonConvert.DeserializeObject<AssetLoaderManifest>(reader.ReadToEnd());
            System.IO.File.WriteAllText($"dump/{name}.json", JsonConvert.SerializeObject(ret, Formatting.Indented)); // Yes, this is a really crappy way to format json
            Console.WriteLine($"Saved manifest to {name}.json");
            return ret;
        }

        static void Main(string[] args)
        {
            var client = new LwClient();
            var verRes = client.Send<VersionGetRequest, VersionGetResponse>(new VersionGetRequest());
            Console.WriteLine(JsonConvert.SerializeObject(verRes));

            var http = new HttpClient();
            (string, string)[] targets =
            {
                ("resource", verRes.resource_path),
                ("master", verRes.master_path),
                ("assetbundle", verRes.assetbundle_path),
            };
            Directory.CreateDirectory("dump");
            foreach (var (name, path) in targets) {
                int cur = 0;
                var manifest = GetManifest(http, name, path);
                Directory.CreateDirectory($"dump/{name}");
                Parallel.ForEach(manifest.AssetInfos, new ParallelOptions { MaxDegreeOfParallelism = 16 }, info =>
                {
                    Console.WriteLine($"({Interlocked.Increment(ref cur)}/{manifest.AssetInfos.Count}) {name}/{info.Name}");

                    var httpReq = new HttpRequestMessage(HttpMethod.Get, $"{path}{info.Name}");
                    using var inStream = http.Send(httpReq).Content.ReadAsStream();
                    using var outStream = System.IO.File.Create($"dump/{name}/{info.Name}");
                    inStream.CopyTo(outStream);
                });
            }
        }
    }
}
