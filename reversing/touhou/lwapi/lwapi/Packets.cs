using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace lwapi
{
    public class SendPacket
    {
        public string? device_id;
        public string? session_id;
        public int resource_version;
        public int app_version;
        public string body;
    }

    public class RecvPacket
    {
        public int status;
        public string now;
        public string? error_message;
        public string? body;
    }

    public abstract class RequestPacket
    {
        protected string m_apiname;
        public string GetAPIName() => m_apiname;
    }

    public abstract class ResponsePacket
    {
        public virtual bool IsEncryptionKeyUpdate() => true;
        public string session_id;
        public string encryption_key;
        public int response_time;
    }

    public class GameInitRequest : RequestPacket
    {
        public GameInitRequest()
        {
            m_apiname = "game_init";
            platform_type = 1;
        }
        public int platform_type;
    }

    public class GameInitResponse : ResponsePacket
    {
        public override bool IsEncryptionKeyUpdate() => false;
        public string api_server;
        public string resource_server;
    }

    public class VersionGetRequest : RequestPacket
    {
        public VersionGetRequest()
        {
            m_apiname = "version_get";
            platform_type = 1;
        }
        public int platform_type;
    }

    public class VersionGetResponse : ResponsePacket
    {
        public override bool IsEncryptionKeyUpdate() => false;
        public int version;
        public string resource_path;
        public string master_path;
        public string assetbundle_path;
        public int resource_version;
    }
}
